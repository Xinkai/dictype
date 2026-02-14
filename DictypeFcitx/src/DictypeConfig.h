#pragma once

#include <fcitx-config/configuration.h>
#include <fcitx-utils/i18n.h>

FCITX_CONFIGURATION(
    DictypeConfig,
    fcitx::KeyListOption triggerKey1{this,
                                     "TriggerKey1",
                                     _("Trigger Key for Profile1"),
                                     {fcitx::Key("Alt+Meta+1")},
                                     fcitx::KeyListConstrain()};

    fcitx::KeyListOption triggerKey2{this,
                                     "TriggerKey2",
                                     _("Trigger Key for Profile2"),
                                     {fcitx::Key("Alt+Meta+2")},
                                     fcitx::KeyListConstrain()};

    fcitx::KeyListOption triggerKey3{this,
                                     "TriggerKey3",
                                     _("Trigger Key for Profile3"),
                                     {fcitx::Key("Alt+Meta+3")},
                                     fcitx::KeyListConstrain()};

    fcitx::KeyListOption triggerKey4{this,
                                     "TriggerKey4",
                                     _("Trigger Key for Profile4"),
                                     {fcitx::Key("Alt+Meta+4")},
                                     fcitx::KeyListConstrain()};

    fcitx::KeyListOption triggerKey5{this,
                                     "TriggerKey5",
                                     _("Trigger Key for Profile5"),
                                     {fcitx::Key("Alt+Meta+5")},
                                     fcitx::KeyListConstrain()};

);
