#include <ranges>
#include <sstream>

#include <fcitx-utils/inputbuffer.h>
#include <fcitx/inputcontext.h>

#include "dictype.grpc.pb.h"

#include "DictypeLog.h"
#include "DictypeState.h"

DictypeState::DictypeState() = default;

void DictypeState::reset() {
    latestCommittableBeginTime_ = 0;
    texts_.clear();
    stage = DictypeStage::Closed;
    errorMsg_.clear();
}

void DictypeState::stop() {
    if (stage == DictypeStage::Connecting ||
        stage == DictypeStage::Transcribing) {
        stage = DictypeStage::Stopping;
    } else {
        DICTYPE_WARN() << "not in connecting or transcribing state.";
    }
}

void DictypeState::setText(const Dictype::TranscribeResponse& response) {
    if (!(stage == DictypeStage::Connecting ||
          stage == DictypeStage::Transcribing)) {
        DICTYPE_WARN() << "not in connecting or transcribing state.";
        return;
    }
    if (stage == DictypeStage::Connecting) {
        stage = DictypeStage::Transcribing;
    }
    const uint32_t beginTime = response.begin_time();
    texts_[beginTime] = response.text();
    if (response.sentence_end()) {
        latestCommittableBeginTime_ =
            std::max(latestCommittableBeginTime_, beginTime);
    }
}

std::string DictypeState::getUncommittedText() const {
    std::ostringstream oss;
    for (const auto& [beginTime, value] : texts_ | std::views::all) {
        if (beginTime > latestCommittableBeginTime_) {
            oss << value;
        }
    }
    return oss.str();
}

std::optional<std::string> DictypeState::takeCommittableText() {
    std::string committed;
    auto it = texts_.begin();
    while (it != texts_.end() && it->first <= latestCommittableBeginTime_) {
        committed += it->second;
        it = texts_.erase(it);
    }
    if (committed.empty()) {
        return std::nullopt;
    }
    return committed;
}

void DictypeState::setError(const std::string& errorMsg) {
    DICTYPE_WARN() << errorMsg;
    if (stage == DictypeStage::Errored) {
        return;
    }
    stage = DictypeStage::Errored;
    errorMsg_ = errorMsg;
}

std::string DictypeState::getErrorMsg() const { return errorMsg_; }
