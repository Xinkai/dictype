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
    EXPECT_EQ(state.getStage(), DictypeStage::Closed);
    EXPECT_EQ(state.getUncommittedText(), "");
    EXPECT_FALSE(state.takeCommittableText().has_value());
}

TEST(DictypeStateTest, NewSessionClearsStateAndData) {
    DictypeState state;

    state.setText(MakeResponse(1, "hello ", true));

    state.clear();

    EXPECT_EQ(state.getStage(), DictypeStage::Closed);
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
    EXPECT_EQ(state.getStage(), DictypeStage::Closed);

    state.setConnecting();
    state.stop();
    EXPECT_EQ(state.getStage(), DictypeStage::Stopping);

    state.setText(MakeResponse(0, "test"));
    state.stop();
    EXPECT_EQ(state.getStage(), DictypeStage::Stopping);
}

TEST(DictypeStateTest, SetWordIgnoredUnlessConnectingOrTranscribing) {
    DictypeState state;
    state.setText(MakeResponse(1, "hello "));
    EXPECT_EQ(state.getStage(), DictypeStage::Closed);
    EXPECT_EQ(state.getUncommittedText(), "");
    EXPECT_FALSE(state.takeCommittableText().has_value());
}

TEST(DictypeStateTest,
     SetWordTransitionsToTranscribingAndTracksCommitBoundaries) {
    DictypeState state;
    state.setConnecting();
    state.setText(MakeResponse(0, "hello ", false));
    EXPECT_EQ(state.getStage(), DictypeStage::Transcribing);
    EXPECT_EQ(state.getUncommittedText(), "hello ");
    EXPECT_EQ(state.takeCommittableText(), std::nullopt);
    state.setText(MakeResponse(0, "hello world!", true));
    EXPECT_EQ(state.getUncommittedText(), "");
    EXPECT_EQ(state.takeCommittableText(), "hello world!");
    EXPECT_EQ(state.takeCommittableText(), std::nullopt);

    state.setText(MakeResponse(10, "HELLO ", false));
    EXPECT_EQ(state.takeCommittableText(), std::nullopt);
    EXPECT_EQ(state.getUncommittedText(), "HELLO ");
    state.setText(MakeResponse(10, "HELLO WORLD!", true));
    EXPECT_EQ(state.takeCommittableText(), "HELLO WORLD!");
    EXPECT_EQ(state.getUncommittedText(), "");
    EXPECT_EQ(state.takeCommittableText(), std::nullopt);
}

TEST(DictypeStateTest, SetErrorStoresFirstErrorAndLocksStage) {
    DictypeState state;
    state.setError("boom");
    EXPECT_EQ(state.getStage(), DictypeStage::Errored);
    EXPECT_EQ(state.getErrorMsg(), "boom");

    state.setError("second");
    EXPECT_EQ(state.getStage(), DictypeStage::Errored);
    EXPECT_EQ(state.getErrorMsg(), "boom");
}

TEST(DictypeStateTest, RetainErrorMessageUntilNewSession) {
    DictypeState state;
    state.setError("boom");
    EXPECT_EQ(state.getStage(), DictypeStage::Errored);
    EXPECT_EQ(state.getErrorMsg(), "boom");

    state.clear();
    EXPECT_EQ(state.getStage(), DictypeStage::Errored);
    EXPECT_EQ(state.getErrorMsg(), "boom");

    state.newSession(nullptr);
    EXPECT_EQ(state.getStage(), DictypeStage::Closed);
    EXPECT_EQ(state.getErrorMsg(), "");
}
