#include <fcitx-config/iniparser.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/event.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>

#include <grpcpp/client_context.h>
#include <grpcpp/create_channel.h>
#include <grpcpp/security/credentials.h>
#include <grpcpp/support/client_callback.h>

#include "dictype.grpc.pb.h"

#include "DictypeFcitx.h"
#include "DictypeLog.h"
#include "DictypeState.h"
#include "GrpcClient.h"

static std::string toHex(const std::array<uint8_t, 16>& data) {
    std::string out;
    out.reserve(32);

    for (uint8_t b : data) {
        out += std::format("{:02x}", b);
    }
    return out;
}

DictypeFcitx::DictypeFcitx(fcitx::AddonManager* addonManager)
    : eventLoop_(addonManager->eventLoop()),
      dispatcher_(addonManager->instance()->eventDispatcher()),
      instance_(addonManager->instance()),
      factory_([](fcitx::InputContext&) { return new DictypeState(); }) {
    DICTYPE_DEBUG() << "created";
    instance_->inputContextManager().registerProperty("dictypeState",
                                                      &factory_);

    eventHandlers_.emplace_back(instance_->watchEvent(
        fcitx::EventType::InputContextKeyEvent,
        fcitx::EventWatcherPhase::Default, [this](fcitx::Event& event) {
            auto& keyEvent = dynamic_cast<fcitx::KeyEvent&>(event);
            if (keyEvent.isRelease()) {
                return;
            }
            if (keyEvent.key().checkKeyList(*config_.triggerKey1)) {
                if (running_.load(std::memory_order_acquire)) {
                    stop_();
                } else {
                    trigger_(keyEvent, "Profile1");
                }
                keyEvent.filterAndAccept();
            } else if (keyEvent.key().checkKeyList(*config_.triggerKey2)) {
                if (running_.load(std::memory_order_acquire)) {
                    stop_();
                } else {
                    trigger_(keyEvent, "Profile2");
                }
                keyEvent.filterAndAccept();
            } else if (keyEvent.key().checkKeyList(*config_.triggerKey3)) {
                if (running_.load(std::memory_order_acquire)) {
                    stop_();
                } else {
                    trigger_(keyEvent, "Profile3");
                }
                keyEvent.filterAndAccept();
            } else if (keyEvent.key().checkKeyList(*config_.triggerKey4)) {
                if (running_.load(std::memory_order_acquire)) {
                    stop_();
                } else {
                    trigger_(keyEvent, "Profile4");
                }
                keyEvent.filterAndAccept();
            } else if (keyEvent.key().checkKeyList(*config_.triggerKey5)) {
                if (running_.load(std::memory_order_acquire)) {
                    stop_();
                } else {
                    trigger_(keyEvent, "Profile5");
                }
                keyEvent.filterAndAccept();
            }
        }));

    const auto cred = grpc::experimental::LocalCredentials(UDS);
    auto channelArgs = grpc::ChannelArguments();
    channelArgs.SetString(GRPC_ARG_DEFAULT_AUTHORITY, "localhost");
    const auto channel =
        grpc::CreateCustomChannel(getServerEndpoint_(), cred, channelArgs);
    auto stubTemp = Dictype::Dictype::NewStub(channel);
    this->stub.swap(stubTemp);
}

DictypeFcitx::~DictypeFcitx() = default;

fcitx::Instance* DictypeFcitx::instance() const { return instance_; }

auto& DictypeFcitx::factory() { return factory_; }

void DictypeFcitx::updateUI_(const DictypeState* state) const {
    const auto inputContext = instance_->lastFocusedInputContext();
    inputContext->inputPanel().reset();
    const std::string uuid = toHex(inputContext->uuid());
    DICTYPE_INFO() << "InputContext: " << uuid << " Update UI "
                   << state->getUncommittedText();

    fcitx::TextFormatFlags format = fcitx::TextFormatFlag::DontCommit;
    const bool clientPreedit =
        inputContext->capabilityFlags().test(fcitx::CapabilityFlag::Preedit);
    if (clientPreedit) {
        format = {fcitx::TextFormatFlag::Underline,
                  fcitx::TextFormatFlag::DontCommit};
    }
    fcitx::Text preedit;

    preedit.append(state->getUncommittedText(), format);
    preedit.setCursor(static_cast<int>(preedit.textLength()));

    if (clientPreedit) {
        inputContext->inputPanel().setClientPreedit(preedit);
    } else {
        inputContext->inputPanel().setPreedit(preedit);
    }
    inputContext->updatePreedit();
    DICTYPE_INFO() << "PreEdit: " << preedit.toString();

    {
        fcitx::Text auxUp;
        switch (state->stage) {
        case DictypeStage::Closed: {
            break;
        }
        case DictypeStage::Connecting: {
            auxUp = fcitx::Text(std::string{"🟡 "} + _("Connecting"));
            break;
        }
        case DictypeStage::Errored: {
            auxUp = fcitx::Text(std::string{"🔴 "} + _("Error: ") +
                                state->getErrorMsg());
            break;
        }
        case DictypeStage::Transcribing: {
            auxUp = fcitx::Text(std::string{"🟢 "} + _("Transcribing"));
            break;
        }
        case DictypeStage::Stopping: {
            auxUp = fcitx::Text(std::string{"🟠 "} + _("Stopping"));
            break;
        }
        default: {
        }
        }
        if (!auxUp.empty()) {
            inputContext->inputPanel().setAuxUp(auxUp);
        }
    }

    inputContext->updateUserInterface(
        fcitx::UserInterfaceComponent::InputPanel);
}

void DictypeFcitx::reloadConfig() { readAsIni(config_, configFile); }

const fcitx::Configuration* DictypeFcitx::getConfig() const { return &config_; }

void DictypeFcitx::setConfig(const fcitx::RawConfig& raw_config) {
    config_.load(raw_config, true);
    safeSaveAsIni(config_, configFile);
}

std::string DictypeFcitx::getServerEndpoint_() {
    uid_t uid = getuid();
    return std::format("unix:///var/run/user/{}/dictyped.socket", uid);
}

void DictypeFcitx::trigger_(const fcitx::KeyEvent& keyEvent,
                            const std::string& profileName) const {
    auto* state = keyEvent.inputContext()->propertyFor(&factory_);
    {
        DICTYPE_INFO() << "Triggered";
        state->reset();
    }

    const auto syncState = [](const DictypeFcitx* that,
                              fcitx::InputContext* inputContext) {
        auto* stateLocal = inputContext->propertyFor(&that->factory_);
        const auto committable = stateLocal->takeCommittableText();
        if (committable.has_value()) {
            const std::string uuid = toHex(inputContext->uuid());
            DICTYPE_INFO() << uuid << "Committing: " << committable.value();
            inputContext->commitString(committable.value());
        }
        that->updateUI_(stateLocal);
    };

    // Start a non-blocking gRPC streaming call to dictyped.
    try {
        // Capture the necessary pointers for updating state and UI.
        const auto onResponse = [ref = this->watch(), syncState](
                                    const Dictype::TranscribeResponse& resp) {
            const auto that = ref.get();
            if (that == nullptr) {
                DICTYPE_WARN() << "instance is gone.";
                return;
            }

            Dictype::TranscribeResponse responseCopy = resp;
            that->dispatcher_.scheduleWithContext(
                ref, [ref, syncState, resp = std::move(responseCopy)]() {
                    const auto that = ref.get();
                    if (that == nullptr) {
                        DICTYPE_WARN() << "instance is gone.";
                        return;
                    }
                    const auto inputContext =
                        that->instance_->lastFocusedInputContext();
                    if (inputContext == nullptr) {
                        DICTYPE_WARN() << "no focused input context.";
                        return;
                    }
                    auto* stateLocal =
                        inputContext->propertyFor(&that->factory_);
                    // Update state text from response and refresh UI.
                    DICTYPE_WARN() << "Resp: " << resp.DebugString();
                    stateLocal->setText(resp);
                    syncState(that, inputContext);
                });
        };

        // When the stream completes (OK or error), clear running_.
        const auto onDone = [ref = this->watch(),
                             syncState](const grpc::Status& s) {
            const auto that = ref.get();
            if (that == nullptr) {
                DICTYPE_WARN() << "instance is gone.";
                return;
            }

            const grpc::Status statusCopy = grpc::Status{s};
            that->dispatcher_.scheduleWithContext(
                ref, [ref, syncState, status = statusCopy]() {
                    const auto that = ref.get();
                    if (that == nullptr) {
                        DICTYPE_WARN() << "instance is gone.";
                        return;
                    }
                    const auto inputContext =
                        that->instance_->lastFocusedInputContext();
                    if (inputContext == nullptr) {
                        DICTYPE_WARN() << "no focused input context.";
                        return;
                    }
                    auto* state2 = inputContext->propertyFor(&that->factory_);

                    if (!status.ok()) {
                        state2->setError(status.error_message());
                        DICTYPE_ERROR() << "stream ended with error: "
                                        << status.error_message();
                    } else {
                        state2->reset();
                        DICTYPE_INFO() << "stream completed.";
                    }
                    syncState(that, inputContext);
                    that->running_.store(false, std::memory_order_release);
                });
        };

        // Reactor deletes itself upon completion; do not hold the pointer.
        (void)new GrpcClient(stub.get(), std::move(onResponse),
                             std::move(onDone), profileName);
        // Mark running only after we've successfully dispatched the stream.
        if (state->stage == DictypeStage::Closed) {
            state->stage = DictypeStage::Connecting;
            running_.store(true, std::memory_order_release);
            DICTYPE_INFO() << "Started transcribe stream.";
        }
    } catch (...) {
        DICTYPE_ERROR() << "Failed to start transcribe stream.";
        state->setError("Unexpected error: failed to start stream.");
    }

    updateUI_(state);
}

void DictypeFcitx::stop_() const {
    const auto inputContext = this->instance()->lastFocusedInputContext();
    auto* state = inputContext->propertyFor(&factory_);

    try {
        auto* ctx = new grpc::ClientContext();
        auto* req = new Dictype::StopRequest();
        auto* resp = new Dictype::StopResponse();

        stub->async()->Stop(
            ctx, req, resp,
            [ref = this->watch(), ctx, req, resp](const grpc::Status& s) {
                const auto that = ref.get();
                if (that == nullptr) {
                    DICTYPE_WARN() << "instance is gone.";
                    return;
                }
                const auto inputContext =
                    that->instance_->lastFocusedInputContext();
                if (inputContext == nullptr) {
                    DICTYPE_WARN() << "no focused input context.";
                    return;
                }
                auto* state2 = inputContext->propertyFor(&that->factory_);
                if (!s.ok()) {
                    DICTYPE_ERROR()
                        << "Stop RPC failed (async): " << s.error_message();
                    state2->setError(s.error_message());
                } else {
                    DICTYPE_INFO()
                        << "Stop RPC ok (async), stopped=" << resp->stopped();
                    state2->stop();
                    that->running_.store(false, std::memory_order_release);
                }
                delete ctx;
                delete req;
                delete resp;
            });

        DICTYPE_INFO() << "Stop RPC dispatched asynchronously.";
    } catch (...) {
        DICTYPE_ERROR() << "Failed to dispatch Stop RPC (async).";
        state->setError("Failed to stop.");
    }
}

class DictypeFcitxFactory final : public fcitx::AddonFactory {
    fcitx::AddonInstance* create(fcitx::AddonManager* manager) override {
        return new DictypeFcitx(manager);
    }
};

FCITX_ADDON_FACTORY_V2(dictype, DictypeFcitxFactory);
