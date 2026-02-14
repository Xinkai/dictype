#include <iomanip>
#include <sstream>

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
    std::ostringstream oss;
    oss << std::hex << std::setfill('0');
    for (uint8_t b : data) {
        oss << std::setw(2) << static_cast<int>(b);
    }
    return oss.str();
}

DictypeFcitx::DictypeFcitx(fcitx::AddonManager* addonManager)
    : eventLoop_(addonManager->eventLoop()),
      dispatcher_(addonManager->instance()->eventDispatcher()),
      instance_(addonManager->instance()) {
    DICTYPE_INFO() << "created";

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

void DictypeFcitx::closeUI_() const {
    const auto inputContextOpt = state_.inputContext();
    if (!inputContextOpt.has_value()) {
        DICTYPE_WARN() << "input context is gone.";
        return;
    }
    auto* inputContext = *inputContextOpt;
    inputContext->inputPanel().reset();

    inputContext->updateUserInterface(
        fcitx::UserInterfaceComponent::InputPanel);
}

void DictypeFcitx::updateUI_() const {
    const auto inputContextOpt = state_.inputContext();
    if (!inputContextOpt.has_value()) {
        DICTYPE_WARN() << "input context is gone.";
        return;
    }
    auto* inputContext = *inputContextOpt;
    inputContext->inputPanel().reset();
    const std::string uuid = toHex(inputContext->uuid());
    DICTYPE_INFO() << "InputContext: " << uuid << " Update UI "
                   << state_.getUncommittedText();

    fcitx::TextFormatFlags format = fcitx::TextFormatFlag::DontCommit;
    const bool clientPreedit =
        inputContext->capabilityFlags().test(fcitx::CapabilityFlag::Preedit);
    if (clientPreedit) {
        format = {fcitx::TextFormatFlag::Underline,
                  fcitx::TextFormatFlag::DontCommit};
    }

    if (const std::string uncommitted = state_.getUncommittedText();
        !uncommitted.empty()) {
        fcitx::Text preedit;
        preedit.append(state_.getUncommittedText(), format);
        preedit.setCursor(static_cast<int>(preedit.textLength()));
        if (clientPreedit) {
            inputContext->inputPanel().setClientPreedit(preedit);
        } else {
            inputContext->inputPanel().setPreedit(preedit);
        }
        inputContext->updatePreedit();
        DICTYPE_INFO() << "PreEdit: " << preedit.toString();
    }

    {
        fcitx::Text auxUp;
        switch (state_.stage) {
        case DictypeStage::Closed: {
            break;
        }
        case DictypeStage::Connecting: {
            auxUp = fcitx::Text(std::string{"🟡 "} + _("Connecting"));
            break;
        }
        case DictypeStage::Errored: {
            auxUp = fcitx::Text(std::string{"🔴 "} + _("Error: ") +
                                state_.getErrorMsg());
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
    const uid_t uid = getuid();
    std::ostringstream oss;
    oss << "unix:///var/run/user/" << uid << "/dictype/dictyped.socket";
    return oss.str();
}

void DictypeFcitx::trigger_(const fcitx::KeyEvent& keyEvent,
                            const std::string& profileName) const {
    auto* inputContext = keyEvent.inputContext();

    {
        DICTYPE_INFO() << "Triggered " << profileName;
        if (!state_.newSession(inputContext)) {
            DICTYPE_ERROR() << "Previous session is not cleared.";
            return;
        }
    }

    const auto syncState =
        [](const DictypeFcitx* that) {
            const auto inputContextOpt = that->state_.inputContext();
            if (!inputContextOpt.has_value()) {
                DICTYPE_WARN() << "input context is gone.";
                if (that->running_.load(std::memory_order_acquire)) {
                    that->stop_();
                }
                return;
            }
            const auto lastFocusedInputContext =
                that->instance()->lastFocusedInputContext();
            auto* inputContext = *inputContextOpt;
            if (lastFocusedInputContext != inputContext) {
                // WORKAROUND: I suspect with some backends, InputContext
                // commitString() always commits to the currently focused text
                // widget, not the text widget associated with InputContext.
                const std::string uuid = toHex(lastFocusedInputContext->uuid());
                DICTYPE_INFO()
                    << "last focused input uuid: " << uuid
                    << ". Different InputContexts detected. Delaying commit...";
                // TODO: watch for the re-focus, and run syncState() again.
                return;
            }
            if (const auto committable = that->state_.takeCommittableText();
                committable.has_value()) {
                const std::string uuid = toHex(inputContext->uuid());
                DICTYPE_INFO()
                    << uuid << " committing: " << committable.value();
                inputContext->commitString(committable.value());
            }
            that->updateUI_();
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
                    const auto that2 = ref.get();
                    if (that2 == nullptr) {
                        DICTYPE_WARN() << "instance is gone.";
                        return;
                    }

                    // Update state text from response and refresh UI.
                    that2->state_.setText(resp);
                    syncState(that2);
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

            const auto statusCopy = grpc::Status{s};
            that->dispatcher_.scheduleWithContext(
                ref, [ref, syncState, status = statusCopy]() {
                    const auto that2 = ref.get();
                    if (that2 == nullptr) {
                        DICTYPE_WARN() << "instance is gone.";
                        return;
                    }

                    if (!status.ok()) {
                        that2->state_.setError(status.error_message());
                        DICTYPE_ERROR() << "stream ended with error: "
                                        << status.error_message();
                    } else {
                        DICTYPE_INFO() << "stream completed.";
                    }

                    syncState(that2);
                    that2->updateUI_();

                    that2->closeUI_();
                    that2->state_.clear();

                    that2->running_.store(false, std::memory_order_release);
                });
        };

        // Reactor deletes itself upon completion; do not hold the pointer.
        (void)new GrpcClient(stub.get(), std::move(onResponse),
                             std::move(onDone), profileName);
        // Mark running only after we've successfully dispatched the stream.
        if (state_.stage == DictypeStage::Closed) {
            state_.stage = DictypeStage::Connecting;
            running_.store(true, std::memory_order_release);
            DICTYPE_INFO() << "Started transcribe stream.";
        }
    } catch (...) {
        DICTYPE_ERROR() << "Failed to start transcribe stream.";
        state_.setError("Unexpected error: failed to start stream.");
    }

    updateUI_();
}

void DictypeFcitx::stop_() const {
    if (state_.stage == DictypeStage::Stopping) {
        DICTYPE_INFO() << "Stop RPC skipped: already stopping.";
        return;
    }

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
                } else {
                    if (!s.ok()) {
                        DICTYPE_ERROR()
                            << "Stop RPC failed (async): " << s.error_message();
                        that->state_.setError(s.error_message());
                    } else {
                        DICTYPE_INFO() << "Stop RPC ok (async), stopped="
                                       << resp->stopped();
                        that->state_.stop();
                    }
                    that->updateUI_();
                }

                delete ctx;
                delete req;
                delete resp;
            });

        DICTYPE_INFO() << "Stop RPC dispatched asynchronously.";
    } catch (...) {
        DICTYPE_ERROR() << "Failed to dispatch Stop RPC (async).";
        state_.setError("Failed to stop.");
        updateUI_();
    }
}

class DictypeFcitxFactory final : public fcitx::AddonFactory {
    fcitx::AddonInstance* create(fcitx::AddonManager* manager) override {
        return new DictypeFcitx(manager);
    }
};

FCITX_ADDON_FACTORY_V2(dictype, DictypeFcitxFactory);
