#pragma once

#include <atomic>

#include <fcitx/action.h>
#include <fcitx/addoninstance.h>
#include <fcitx/inputcontext.h>

#include "DictypeConfig.h"

class DictypeState;

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
    auto& factory();

    void reloadConfig() override;
    const fcitx::Configuration* getConfig() const override;
    void setConfig(const fcitx::RawConfig& /*unused*/) override;

  private:
    fcitx::EventLoop* eventLoop_;
    fcitx::EventDispatcher& dispatcher_;

    fcitx::Instance* instance_;
    fcitx::FactoryFor<DictypeState> factory_;
    DictypeConfig config_;

    static std::string getServerEndpoint_();
    std::unique_ptr<Dictype::Dictype::Stub> stub;
    void updateUI_(const DictypeState* state) const;

    mutable std::atomic<bool> running_{false};
    void trigger_(const fcitx::KeyEvent& keyEvent,
                  const std::string& profileName) const;
    void stop_() const;

    std::vector<std::unique_ptr<fcitx::HandlerTableEntry<fcitx::EventHandler>>>
        eventHandlers_;
};
