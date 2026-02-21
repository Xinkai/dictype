#include <sstream>

#include <fcitx-utils/inputbuffer.h>
#include <fcitx/inputcontext.h>

#include "dictype.grpc.pb.h"

#include "DictypeLog.h"
#include "DictypeState.h"

DictypeState::DictypeState() = default;

void DictypeState::clear() {
    latestCommittableBeginTime_ = -1;
    if (!texts_.empty()) {
        DICTYPE_WARN() << "uncommitted texts: " << getUncommittedText();
        texts_.clear();
    }
    cleared_ = true;
}

bool DictypeState::newSession(fcitx::InputContext* inputContext) {
    if (!cleared_) {
        DICTYPE_WARN() << "Previous session is not cleared.";
        return false;
    }
    if (stage_ == DictypeStage::Errored) {
        stage_ = DictypeStage::Closed;
        errorMsg_.clear();
        inputContext_.unwatch();
    }
    if (inputContext == nullptr) {
        inputContext_.unwatch();
    } else {
        inputContext_ = inputContext->watch();
    }
    cleared_ = false;
    return true;
}

void DictypeState::stop() {
    if (stage_ == DictypeStage::Connecting ||
        stage_ == DictypeStage::Transcribing) {
        stage_ = DictypeStage::Stopping;
    } else {
        DICTYPE_WARN() << "not in connecting or transcribing state.";
    }
}

void DictypeState::setConnecting() {
    if (stage_ != DictypeStage::Closed) {
        DICTYPE_WARN() << "not in closed state.";
        return;
    }
    stage_ = DictypeStage::Connecting;
}

void DictypeState::setText(const Dictype::TranscribeResponse& response) {
    if (!(stage_ == DictypeStage::Connecting ||
          stage_ == DictypeStage::Transcribing ||
          stage_ == DictypeStage::Stopping)) {
        DICTYPE_WARN()
            << "not in connecting or transcribing or stopping state.";
        return;
    }
    if (stage_ == DictypeStage::Connecting) {
        stage_ = DictypeStage::Transcribing;
    }
    const uint32_t beginTime = response.begin_time();
    texts_[beginTime] = response.text();
    if (response.sentence_end()) {
        latestCommittableBeginTime_ = std::max(latestCommittableBeginTime_,
                                               static_cast<int64_t>(beginTime));
    }
}

std::string DictypeState::getUncommittedText() const {
    std::ostringstream oss;
    for (const auto& [beginTime, value] : texts_) {
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
    if (stage_ == DictypeStage::Errored) {
        return;
    }
    stage_ = DictypeStage::Errored;
    errorMsg_ = errorMsg;
}

std::string DictypeState::getErrorMsg() const { return errorMsg_; }

std::optional<fcitx::InputContext*> DictypeState::inputContext() const {
    if (auto* inputContext = inputContext_.get(); inputContext != nullptr) {
        return inputContext;
    }
    return std::nullopt;
}
