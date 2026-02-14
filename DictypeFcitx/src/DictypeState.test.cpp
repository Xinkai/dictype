#include <optional>
#include <string>

#include <gtest/gtest.h>

#include "DictypeState.h"
#include "dictype.grpc.pb.h"

namespace {
Dictype::TranscribeResponse MakeResponse(uint32_t beginTime,
                                         const std::string& text,
                                         bool sentenceEnd = false) {
    Dictype::TranscribeResponse response;
    response.set_begin_time(beginTime);
    response.set_text(text);
    response.set_sentence_end(sentenceEnd);
    return response;
}
} // namespace

TEST(DictypeStateTest, DefaultStateIsClosed) {
    DictypeState state;
    EXPECT_EQ(state.stage, DictypeStage::Closed);
    EXPECT_EQ(state.getUncommittedText(), "");
    EXPECT_FALSE(state.takeCommittableText().has_value());
}

TEST(DictypeStateTest, NewSessionClearsStateAndData) {
    DictypeState state;
    state.stage = DictypeStage::Connecting;
    state.setError("oops");
    state.stage = DictypeStage::Transcribing;
    state.setText(MakeResponse(1, "hello ", true));

    state.clear();

    EXPECT_EQ(state.stage, DictypeStage::Closed);
    EXPECT_EQ(state.getErrorMsg(), "");
    EXPECT_EQ(state.getUncommittedText(), "");
    EXPECT_FALSE(state.takeCommittableText().has_value());
}

TEST(DictypeStateTest, NewSessionRequiresPreviousClear) {
    DictypeState state;
    EXPECT_TRUE(state.newSession(nullptr));
    EXPECT_FALSE(state.newSession(nullptr));
    state.clear();
    EXPECT_TRUE(state.newSession(nullptr));
}

TEST(DictypeStateTest, StopTransitionsOnlyFromConnectingOrTranscribing) {
    DictypeState state;
    state.stop();
    EXPECT_EQ(state.stage, DictypeStage::Closed);

    state.stage = DictypeStage::Connecting;
    state.stop();
    EXPECT_EQ(state.stage, DictypeStage::Stopping);

    state.stage = DictypeStage::Transcribing;
    state.stop();
    EXPECT_EQ(state.stage, DictypeStage::Stopping);
}

TEST(DictypeStateTest, SetWordIgnoredUnlessConnectingOrTranscribing) {
    DictypeState state;
    state.setText(MakeResponse(1, "hello "));
    EXPECT_EQ(state.stage, DictypeStage::Closed);
    EXPECT_EQ(state.getUncommittedText(), "");
    EXPECT_FALSE(state.takeCommittableText().has_value());
}

TEST(DictypeStateTest,
     SetWordTransitionsToTranscribingAndTracksCommitBoundaries) {
    DictypeState state;
    state.stage = DictypeStage::Connecting;

    state.setText(MakeResponse(1, "hello ", true));
    EXPECT_EQ(state.stage, DictypeStage::Transcribing);
    EXPECT_EQ(state.getUncommittedText(), "");
    auto committable = state.takeCommittableText();
    ASSERT_TRUE(committable.has_value());
    EXPECT_EQ(*committable, "hello ");

    state.setText(MakeResponse(3, "world"));
    EXPECT_EQ(state.getUncommittedText(), "world");
    committable = state.takeCommittableText();
    EXPECT_FALSE(committable.has_value());
}

TEST(DictypeStateTest, SetErrorStoresFirstErrorAndLocksStage) {
    DictypeState state;
    state.stage = DictypeStage::Connecting;
    state.setError("boom");
    EXPECT_EQ(state.stage, DictypeStage::Errored);
    EXPECT_EQ(state.getErrorMsg(), "boom");

    state.setError("second");
    EXPECT_EQ(state.stage, DictypeStage::Errored);
    EXPECT_EQ(state.getErrorMsg(), "boom");
}
