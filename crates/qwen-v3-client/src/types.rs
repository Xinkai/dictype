#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[serde(rename = "zh")]
    Mandarin,

    #[serde(rename = "yue")]
    Cantonese,

    #[serde(rename = "en")]
    English,

    #[serde(rename = "ja")]
    Japanese,

    #[serde(rename = "de")]
    German,

    #[serde(rename = "ko")]
    Korean,

    #[serde(rename = "ru")]
    Russian,

    #[serde(rename = "fr")]
    French,

    #[serde(rename = "pt")]
    Portuguese,

    #[serde(rename = "ar")]
    Arabic,

    #[serde(rename = "it")]
    Italian,

    #[serde(rename = "es")]
    Spanish,

    #[serde(rename = "hi")]
    Hindi,

    #[serde(rename = "id")]
    Indonesian,

    #[serde(rename = "th")]
    Thai,

    #[serde(rename = "tr")]
    Turkish,

    #[serde(rename = "uk")]
    Ukrainian,

    #[serde(rename = "vi")]
    Vietnamese,

    #[serde(rename = "cs")]
    Czech,

    #[serde(rename = "da")]
    Danish,

    #[serde(rename = "fil")]
    Filipino,

    #[serde(rename = "fi")]
    Finnish,

    #[serde(rename = "is")]
    Icelandic,

    #[serde(rename = "ms")]
    Malay,

    #[serde(rename = "no")]
    Norwegian,

    #[serde(rename = "pl")]
    Polish,

    #[serde(rename = "sv")]
    Swedish,
}

#[allow(dead_code)]
pub mod error {
    #[derive(Debug, serde::Deserialize)]
    enum Type {
        #[serde(rename = "error")]
        Error,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Error {
        r#type: Type,
        code: String,
        message: String,
        param: String,
        event_id: String,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Response {
        event_id: String,
        r#type: Type,
        error: Error,
    }
}

#[allow(dead_code)]
pub mod session {
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    enum TurnDetectionType {
        #[serde(rename = "server_vad")]
        ServerVad,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct TurnDetection {
        r#type: TurnDetectionType,
        threshold: f32,
        silence_duration_ms: u32,
    }

    pub mod created {
        #[derive(Debug, serde::Deserialize)]
        enum r#Type {
            #[serde(rename = "session.created")]
            SessionCreated,
        }

        #[derive(Debug, serde::Deserialize)]
        enum SessionObject {
            #[serde(rename = "realtime.session")]
            RealtimeSession,
        }

        #[derive(Debug, serde::Deserialize)]
        struct InputAudioTranscription {}

        #[derive(Debug, serde::Deserialize)]
        struct Session {
            id: String,
            object: SessionObject,
            model: String,
            modalities: Vec<String>,
            input_audio_format: String,
            input_audio_transcription: InputAudioTranscription,
            turn_detection: super::TurnDetection,
        }

        pub mod response {
            use crate::types::session::created::{Session, Type};

            #[derive(Debug, serde::Deserialize)]
            pub struct Response {
                r#type: r#Type,
                event_id: String,
                session: Session,
            }
        }
    }

    pub mod update {
        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        enum Type {
            #[serde(rename = "session.update")]
            SessionUpdate,
        }

        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        enum InputAudioFormat {
            #[serde(rename = "pcm")]
            Pcm,
            #[serde(rename = "opus")]
            Opus,
        }

        pub mod request {
            use crate::config::QwenV3Config;
            use crate::types::Language;

            #[derive(Debug, serde::Serialize)]
            struct InputAudioTranscription {
                language: Language,
            }

            #[derive(Debug, serde::Serialize)]
            struct Session {
                input_audio_format: super::InputAudioFormat,
                sample_rate: u32,
                #[serde(skip_serializing_if = "Option::is_none")]
                input_audio_transcription: Option<InputAudioTranscription>,
                #[serde(skip_serializing_if = "Option::is_none")]
                turn_detection: Option<super::super::TurnDetection>,
            }

            #[derive(Debug, serde::Serialize)]
            pub struct Request {
                event_id: String,
                r#type: super::Type,
                session: Session,
            }

            impl Request {
                pub fn new(event_id: u32, config: &QwenV3Config) -> Self {
                    let mut session = Session {
                        input_audio_format: super::InputAudioFormat::Pcm,
                        input_audio_transcription: None,
                        sample_rate: 16000,
                        turn_detection: None,
                    };
                    if let Some(language) = config.language {
                        session.input_audio_transcription =
                            InputAudioTranscription { language }.into();
                    }

                    if let Some(turn_detection) = &config.turn_detection {
                        session.turn_detection = super::super::TurnDetection {
                            r#type: super::super::TurnDetectionType::ServerVad,
                            silence_duration_ms: turn_detection.silence_duration_ms,
                            threshold: turn_detection.threshold,
                        }
                        .into();
                    }

                    Self {
                        event_id: format!("session_update_{event_id}"),
                        r#type: super::Type::SessionUpdate,
                        session,
                    }
                }
            }
        }

        #[derive(Debug, serde::Deserialize)]
        enum Object {
            #[serde(rename = "realtime.session")]
            RealtimeSession,
        }

        pub mod response {
            use crate::types::Language;
            use crate::types::session::TurnDetectionType;

            #[derive(Debug, serde::Deserialize)]
            enum Type {
                #[serde(rename = "session.updated")]
                SessionUpdated,
            }

            #[derive(Debug, serde::Deserialize)]
            struct TurnDetection {
                r#type: TurnDetectionType,
                threshold: f32,
                silence_duration_ms: u32,
                create_response: bool,
                interrupt_response: bool,
            }

            #[derive(Debug, serde::Deserialize)]
            struct Session {
                id: String,
                object: super::Object,
                model: String,
                modalities: Vec<String>,
                input_audio_format: super::InputAudioFormat,
                input_audio_transcription: InputAudioTranscription,
                turn_detection: TurnDetection,
                sample_rate: u32,
            }

            #[derive(Debug, serde::Deserialize)]
            struct InputAudioTranscription {
                model: String,
                language: Option<Language>,
            }

            #[derive(Debug, serde::Deserialize)]
            pub struct Response {
                event_id: String,
                r#type: Type,
                session: Session,
            }
        }
    }

    pub mod finish {
        pub mod request {
            #[derive(Debug, serde::Serialize)]
            enum Type {
                #[serde(rename = "session.finish")]
                SessionFinish,
            }

            #[derive(Debug, serde::Serialize)]
            pub struct Request {
                event_id: String,
                r#type: Type,
            }

            impl Request {
                pub fn new(event_id: u32) -> Self {
                    Self {
                        event_id: format!("session_finish_req_{event_id}"),
                        r#type: Type::SessionFinish,
                    }
                }
            }
        }
    }

    pub mod finished {
        #[derive(Debug, serde::Deserialize)]
        enum Type {
            #[serde(rename = "session.finished")]
            SessionFinished,
        }

        #[derive(Debug, serde::Deserialize)]
        pub struct Response {
            event_id: String,
            r#type: Type,
        }
    }
}

#[allow(dead_code)]
pub mod input_audio_buffer {
    pub mod append {
        pub mod request {
            use base64::Engine;
            use tokio_util::bytes::Bytes;

            #[derive(Debug, serde::Serialize)]
            enum Type {
                #[serde(rename = "input_audio_buffer.append")]
                InputAudioBufferAppend,
            }

            #[derive(Debug, serde::Serialize)]
            pub struct Request {
                r#type: Type,
                event_id: String,
                audio: String,
            }

            impl Request {
                pub fn new(event_id: impl Into<String>, audio_bytes: Bytes) -> Self {
                    let audio = base64::engine::general_purpose::STANDARD.encode(audio_bytes);
                    Self {
                        r#type: Type::InputAudioBufferAppend,
                        event_id: event_id.into(),
                        audio,
                    }
                }
            }
        }
    }

    pub mod speech_started {
        pub mod response {
            #[derive(Debug, serde::Deserialize)]
            enum InputAudioBufferSpeechStarted {
                #[serde(rename = "input_audio_buffer.speech_started")]
                InputAudioBufferSpeechStarted,
            }
            #[derive(Debug, serde::Deserialize)]
            pub struct Response {
                event_id: String,
                r#type: InputAudioBufferSpeechStarted,
                pub audio_start_ms: u32,
                item_id: String,
            }
        }
    }

    pub mod speech_stopped {
        pub mod response {
            #[derive(Debug, serde::Deserialize)]
            enum InputAudioBufferSpeechStopped {
                #[serde(rename = "input_audio_buffer.speech_stopped")]
                InputAudioBufferSpeechStopped,
            }
            #[derive(Debug, serde::Deserialize)]
            pub struct Response {
                event_id: String,
                r#type: InputAudioBufferSpeechStopped,
                audio_end_ms: u32,
                item_id: String,
            }
        }
    }

    pub mod committed {
        pub mod response {
            #[derive(Debug, serde::Deserialize)]
            enum InputAudioBufferCommitted {
                #[serde(rename = "input_audio_buffer.committed")]
                InputAudioBufferCommitted,
            }

            #[derive(Debug, serde::Deserialize)]
            pub struct Response {
                event_id: String,
                r#type: InputAudioBufferCommitted,
                item_id: String,
            }
        }
    }
}

#[allow(dead_code)]
pub mod conversation {
    pub mod item {
        pub mod input_audio_transcription {
            pub mod completed {
                use crate::types::Language;

                #[derive(Debug, serde::Deserialize)]
                enum Type {
                    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
                    InputAudioBufferConversationItemInputAudioTranscriptionCompleted,
                }

                #[derive(Debug, serde::Deserialize)]
                struct InputTokensDetails {
                    text_tokens: u32,
                    audio_tokens: u32,
                }

                #[derive(Debug, serde::Deserialize)]
                struct OutputTokensDetails {
                    text_tokens: u32,
                }

                #[derive(Debug, serde::Deserialize)]
                struct Usage {
                    duration: u32,
                    total_tokens: u32,
                    input_tokens: u32,
                    output_tokens: u32,
                    input_tokens_details: InputTokensDetails,
                    output_tokens_details: OutputTokensDetails,
                }

                #[derive(Debug, serde::Deserialize)]
                pub struct Response {
                    event_id: String,
                    r#type: Type,
                    item_id: String,
                    content_index: usize,
                    pub transcript: String,
                    language: Language,
                    emotion: String,
                }
            }

            pub mod text {
                use crate::types::Language;

                #[derive(Debug, serde::Deserialize)]
                enum Type {
                    #[serde(rename = "conversation.item.input_audio_transcription.text")]
                    ConversationItemInputAudioTranscriptionText,
                }

                #[derive(Debug, serde::Deserialize)]
                pub struct Response {
                    event_id: String,
                    r#type: Type,
                    item_id: String,
                    content_index: u32,
                    pub text: String,
                    pub language: Language,
                    emotion: String,
                }
            }
        }

        pub mod created {
            #[derive(Debug, serde::Deserialize)]
            enum Type {
                #[serde(rename = "conversation.item.created")]
                ConversationItemCreated,
            }

            #[derive(Debug, serde::Deserialize)]
            enum Object {
                #[serde(rename = "realtime.item")]
                RealtimeItem,
            }

            #[derive(Debug, serde::Deserialize)]
            enum ItemType {
                #[serde(rename = "message")]
                Message,
            }

            #[derive(Debug, serde::Deserialize)]
            enum ItemStatus {
                #[serde(rename = "in_progress")]
                InProgress,
            }

            #[derive(Debug, serde::Deserialize)]
            enum ItemRole {
                #[serde(rename = "assistant")]
                Assistant,
            }

            #[derive(Debug, serde::Deserialize)]
            enum ContentType {
                #[serde(rename = "input_audio")]
                InputAudio,
            }

            #[derive(Debug, serde::Deserialize)]
            struct Content {
                r#type: ContentType,
            }

            #[derive(Debug, serde::Deserialize)]
            pub struct Item {
                pub id: String,
                object: Object,
                r#type: ItemType,
                status: ItemStatus,
                role: ItemRole,
                content: Vec<Content>,
            }

            #[derive(Debug, serde::Deserialize)]
            pub struct Response {
                event_id: String,
                r#type: Type,
                pub item: Item,
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ServerEvent {
    SessionCreated(session::created::response::Response),
    SessionUpdated(session::update::response::Response),
    SessionFinished(session::finished::Response),
    ConversationItemCreated(conversation::item::created::Response),
    ConversationItemInputAudioTranscriptionTranscriptionText(
        conversation::item::input_audio_transcription::text::Response,
    ),
    ConversationItemInputAudioTranscriptionCompleted(
        conversation::item::input_audio_transcription::completed::Response,
    ),
    InputAudioBufferSpeechStarted(input_audio_buffer::speech_started::response::Response),
    InputAudioBufferSpeechStopped(input_audio_buffer::speech_stopped::response::Response),
    InputAudioBufferCommitted(input_audio_buffer::committed::response::Response),
    Error(error::Response),
}
