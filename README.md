Dictype
=======

Real-time voice-to-text input on Linux.

Features
--------

- Fcitx integration: customizable trigger keys for your profiles.
- Real-time dictation: no need to wait for a connection before you speak, with real-time preview as the model revises.
- Model
  options: [paraformer-realtime-v2 (Alibaba Cloud)](https://help.aliyun.com/zh/model-studio/real-time-speech-recognition#ea9240a128roy), [qwen3-asr-flash-realtime (Alibaba Cloud)](https://help.aliyun.com/zh/model-studio/qwen-real-time-speech-recognition).

Setup
-----

1. Install packages for your distro.

   <details>
   <summary>Arch Linux</summary>

   <p>
   Install packages from AUR:
   </p>
   <ul>
      <li><a href="https://aur.archlinux.org/packages/dictype-fcitx">dictype-fcitx</a></li>
      <li><a href="https://aur.archlinux.org/packages/dictype">dictype</a></li>
   </ul>
   </details>

2. Configure Dictype.

   ```toml
   # This is the configuration file for Dictype.
   # Put it at `~/.config/dictype.toml`.
   
   [PulseAudio]
   # Use the following command to get a list of available `device_name`.
   # $ pactl --format json list sources \
   #   | jq '[
   #     .[]
   #     | select((.monitor_of_sink == null) and (.name | endswith(".monitor") | not))
   #     | {
   #         device_name: .properties["device.name"],
   #         device_alias: .properties["device.alias"],
   #         device_description: .properties["device.description"]
   #       }
   #   ]'
   preferred_device = "..." # optional
   
   # You can have up to 5 profiles at the same time, starting with Profile1.
   # Each profile may have different formats depending on the model (Backend).
   [Profile1]
   Backend = "ParaformerV2"
   Config = {
       dashscope_api_key = "...",                   # required
       dashscope_websocket_url = "wss://dashscope.aliyuncs.com/api-ws/v1/inference", # optional
       disfluency_removal_enabled = true,           # optional
       language_hints = ["zh"],                     # optional
       semantic_punctuation_enabled = false,        # optional
       max_sentence_silence = 800,                  # optional
       multi_threshold_mode_enabled = false,        # optional
       punctuation_prediction_enabled = true,       # optional
       inverse_text_normalization_enabled = true,   # optional
   }
   
   [Profile2]
   Backend = "QwenV3"
   Config = {
       dashscope_api_key = "...",                                       # required
       dashscope_websocket_url = "wss://dashscope.aliyuncs.com/api-ws/v1/realtime?model=qwen3-asr-flash-realtime", # optional
       language = "en",                                                 # optional
       turn_detection = { threshold = 0.2, silence_duration_ms = 900 }, # optional
   }
   ```

3. Run daemon

   ```bash
   systemctl --user enable dictyped --now
   ```

4. Restart Fcitx.

   > Restarting Fcitx can be complex depending on your setup. The easist way to do this is just restart your computer.

5. Configure `dictype-fcitx` trigger keys using the official Fcitx configuration, under `Configuration addons...`

6. Focus on your text input, then press the profile trigger key to start. Press it again to stop. You may lose focus
   while transcribing.

Requirements
------------

1. `PulseAudio`, or `PipeWire` with pulseaudio compatibility support.
2. `fcitx5`.
3. cloud accounts for respective models (currently supports two models on Alibaba Cloud).

TODOs
-----

- [ ] GUI configuration tool
- [ ] local inference

Disclaimer
----------

- This is a personal project and is not affiliated with any cloud providers or model providers.
- Discretion is advised when it comes to model fees and privacy concerns when using cloud models.

License
-------

MIT License.
