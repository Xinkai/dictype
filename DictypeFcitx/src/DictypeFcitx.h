#pragma once

#include <atomic>

#include <fcitx-utils/trackableobject.h>
#include <fcitx/action.h>
#include <fcitx/addoninstance.h>

#include "DictypeConfig.h"
#include "DictypeState.h"

namespace fcitx {
class Instance;
}

class DictypeFcitx final : public fcitx::AddonInstance,
                           public fcitx::TrackableObject<DictypeFcitx> {
  public:
    static constexpr char configFile[] = "conf/dictype.conf";
    explicit DictypeFcitx(fcitx::AddonManager* addonManager);
    ~DictypeFcitx() override;

    [[nodiscard]] fcitx::Instance* instance() const;

    void reloadConfig() override;
    const fcitx::Configuration* getConfig() const override;
    void setConfig(const fcitx::RawConfig& /*unused*/) override;

  private:
    fcitx::EventLoop* eventLoop_;
    fcitx::EventDispatcher& dispatcher_;

    fcitx::Instance* instance_;
    DictypeConfig config_;

    static std::string getServerEndpoint_();
    std::unique_ptr<Dictype::Dictype::Stub> stub;

    void closeUI_() const;
    void updateUI_() const;

    /**
     * if there is a request currently working
     */
    mutable std::atomic<bool> running_{false};

    /**
     * holds information about requests
     */
    mutable DictypeState state_;

    /**
     * starts transcribing
     */
    void trigger_(const fcitx::KeyEvent& keyEvent,
                  const std::string& profileName) const;

    /**
     * signals the daemon to stop transcribing.
     *
     * note: the work is not done yet, the remaining recognition still needs to
     * be drained.
     */
    void stop_() const;

    std::vector<std::unique_ptr<fcitx::HandlerTableEntry<fcitx::EventHandler>>>
        eventHandlers_;
};
