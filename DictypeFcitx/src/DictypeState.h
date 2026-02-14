#pragma once

#include <map>
#include <optional>
#include <string>

#include <fcitx-utils/inputbuffer.h>
#include <fcitx-utils/trackableobject.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextproperty.h>

enum class DictypeStage {
    Closed,
    Connecting,
    Errored,
    Transcribing,
    Stopping,
};

namespace Dictype {
class TranscribeResponse;
}

class DictypeState final : public fcitx::InputContextProperty {
  public:
    explicit DictypeState();

    void clear();
    bool newSession(fcitx::InputContext* inputContext);

    void stop();

    /**
     * create / update `text` at beginTime
     */
    void setText(const Dictype::TranscribeResponse&);

    [[nodiscard]] std::string getUncommittedText() const;
    std::optional<std::string> takeCommittableText();

    DictypeStage stage{DictypeStage::Closed};

    void setError(const std::string& errorMsg);
    [[nodiscard]] std::string getErrorMsg() const;

    [[nodiscard]] std::optional<fcitx::InputContext*> inputContext() const;

  private:
    uint32_t latestCommittableBeginTime_ = 0;
    std::map<uint32_t, std::string> texts_;
    std::string errorMsg_;
    fcitx::TrackableObjectReference<fcitx::InputContext> inputContext_;
    bool cleared_ = true;
};
