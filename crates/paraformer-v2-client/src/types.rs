#[allow(dead_code)]
use base_client::grpc_server::TranscribeResponse;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct EmptyObj {}

#[derive(Debug, serde::Serialize)]
pub enum Streaming {
    #[serde(rename = "duplex")]
    Duplex,
}

#[allow(dead_code)]
pub mod run_task {
    pub mod request {
        use super::super::{EmptyObj, Streaming};
        use crate::config::ParaformerV2Config;
        use uuid::Uuid;

        #[derive(Debug, serde::Serialize)]
        pub struct RequestHeader {
            action: &'static str,
            pub task_id: String,
            streaming: Streaming,
        }

        #[derive(Debug, serde::Serialize)]
        enum Format {
            #[serde(rename = "pcm")]
            Pcm,
        }

        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub enum Language {
            #[serde(rename = "zh")]
            Mandarin,
            #[serde(rename = "en")]
            English,
            #[serde(rename = "ja")]
            Japanese,
            #[serde(rename = "yue")]
            Cantonese,
            #[serde(rename = "ko")]
            Korean,
            #[serde(rename = "de")]
            German,
            #[serde(rename = "fr")]
            French,
            #[serde(rename = "ru")]
            Russian,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct RequestPayloadParameters {
            format: Format,
            sample_rate: u32,
            vocabulary_id: Option<String>,
            disfluency_removal_enabled: Option<bool>,
            language_hints: Option<Vec<Language>>,
            semantic_punctuation_enabled: Option<bool>,
            max_sentence_silence: Option<u32>,
            multi_threshold_mode_enabled: Option<bool>,
            punctuation_prediction_enabled: Option<bool>,
            heartbeat: Option<bool>,
            inverse_text_normalization_enabled: Option<bool>,
        }

        impl RequestPayloadParameters {
            fn new(config: &ParaformerV2Config) -> Self {
                let mut parameters = Self {
                    format: Format::Pcm,
                    sample_rate: 16000,
                    vocabulary_id: None,
                    disfluency_removal_enabled: None,
                    language_hints: None,
                    semantic_punctuation_enabled: None,
                    max_sentence_silence: None,
                    multi_threshold_mode_enabled: None,
                    punctuation_prediction_enabled: None,
                    heartbeat: None,
                    inverse_text_normalization_enabled: None,
                };
                if let Some(disfluency_removal_enabled) = config.disfluency_removal_enabled {
                    parameters.disfluency_removal_enabled = Some(disfluency_removal_enabled);
                }
                if let Some(language_hints) = &config.language_hints
                    && !language_hints.is_empty()
                {
                    parameters.language_hints = Some(language_hints.clone());
                }
                if let Some(semantic_punctuation_enabled) = config.semantic_punctuation_enabled {
                    parameters.semantic_punctuation_enabled = Some(semantic_punctuation_enabled);
                }
                if let Some(max_sentence_silence) = config.max_sentence_silence {
                    parameters.max_sentence_silence = Some(max_sentence_silence);
                }
                if let Some(multi_threshold_mode_enabled) = config.multi_threshold_mode_enabled {
                    parameters.multi_threshold_mode_enabled = Some(multi_threshold_mode_enabled);
                }
                if let Some(punctuation_prediction_enabled) = config.punctuation_prediction_enabled
                {
                    parameters.punctuation_prediction_enabled =
                        Some(punctuation_prediction_enabled);
                }
                if let Some(inverse_text_normalization_enabled) =
                    config.inverse_text_normalization_enabled
                {
                    parameters.inverse_text_normalization_enabled =
                        Some(inverse_text_normalization_enabled);
                }

                parameters
            }
        }

        #[derive(Debug, serde::Serialize)]
        pub struct RequestPayload {
            task_group: &'static str,
            task: &'static str,
            function: &'static str,
            model: &'static str,
            parameters: RequestPayloadParameters,
            input: EmptyObj,
        }

        impl RequestPayload {
            #[must_use]
            pub fn new(config: &ParaformerV2Config) -> Self {
                Self {
                    task_group: "audio",
                    task: "asr",
                    function: "recognition",
                    model: "paraformer-realtime-v2",
                    parameters: RequestPayloadParameters::new(config),
                    input: EmptyObj::default(),
                }
            }
        }

        #[derive(Debug, serde::Serialize)]
        pub struct Request {
            pub header: RequestHeader,
            payload: RequestPayload,
        }

        impl Request {
            #[must_use]
            pub fn new(config: ParaformerV2Config) -> Self {
                Self {
                    header: RequestHeader {
                        action: "run-task",
                        task_id: Uuid::new_v4().into(),
                        streaming: Streaming::Duplex,
                    },
                    payload: RequestPayload::new(&config),
                }
            }
        }
    }

    pub mod response {
        use super::super::EmptyObj;

        #[derive(Debug, serde::Deserialize)]
        enum Event {
            #[serde(rename = "task-started")]
            TaskStarted,
        }

        #[derive(Debug, serde::Deserialize)]
        struct Header {
            task_id: String,
            event: Event,
            attributes: EmptyObj,
        }

        #[derive(Debug, serde::Deserialize)]
        pub struct Response {
            header: Header,
            payload: EmptyObj,
        }
    }
}

#[allow(dead_code)]
pub mod result_generated {
    use super::EmptyObj;

    #[derive(Debug, serde::Deserialize)]
    pub enum Event {
        #[serde(rename = "result-generated")]
        ResultGenerated,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Header {
        pub task_id: String,
        pub event: Event,
        attributes: EmptyObj,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Response {
        pub header: Header,
        pub payload: Payload,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct ParaformerWord {
        pub begin_time: u32,
        pub end_time: u32,
        pub text: String,
        pub punctuation: Option<String>,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct ParaformerSentence {
        pub begin_time: u32,
        pub end_time: Option<u32>,
        pub text: String,
        pub heartbeat: Option<bool>,
        pub sentence_end: bool,
        pub words: Vec<ParaformerWord>,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct PayloadOutput {
        pub sentence: ParaformerSentence,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Usage {
        pub duration: u32,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Payload {
        pub output: PayloadOutput,
        pub usage: Option<Usage>,
    }

    #[cfg(test)]
    mod tests {
        use super::Response;

        #[test]
        fn result_generated_deserialize() {
            let data = r#"
        {
          "header": {
            "task_id": "2bf83b9a-baeb-4fda-8d9a-xxxxxxxxxxxx",
            "event": "result-generated",
            "attributes": {}
          },
          "payload": {
            "output": {
              "sentence": {
                "begin_time": 170,
                "end_time": null,
                "text": "好，我知道了",
                "heartbeat": false,
                "sentence_end": true,
                "emo_tag": "neutral",
                "emo_confidence": 0.914,
                "words": [
                  {
                    "begin_time": 170,
                    "end_time": 295,
                    "text": "好",
                    "punctuation": "，"
                  },
                  {
                    "begin_time": 295,
                    "end_time": 503,
                    "text": "我",
                    "punctuation": ""
                  },
                  {
                    "begin_time": 503,
                    "end_time": 711,
                    "text": "知道",
                    "punctuation": ""
                  },
                  {
                    "begin_time": 711,
                    "end_time": 920,
                    "text": "了",
                    "punctuation": ""
                  }
                ]
              }
            },
            "usage": {
              "duration": 3
            }
          }
        }
        "#;
            let _: Response = serde_json::from_str(data).unwrap();
        }

        #[test]
        fn result_generated_deserialize2() {
            let data = r#"
                {"header":{"task_id":"239a50e2-65aa-4c66-bcf1-741c15e7021b","event":"result-generated","attributes":{}},"payload":{"output":{"sentence":{"sentence_id":1,"begin_time":1940,"end_time":null,"text":"","channel_id":0,"speaker_id":null,"sentence_end":false,"sentence_begin":true,"words":[]}}}}
                "#;

            let _: Response = serde_json::from_str(data).unwrap();
        }
    }
}

#[allow(dead_code)]
pub mod finish_task {
    pub mod request {
        use super::super::{EmptyObj, Streaming};

        #[derive(Debug, Default, serde::Serialize)]
        struct Payload {
            pub input: EmptyObj,
        }

        #[derive(Debug, serde::Serialize)]
        struct Header {
            pub action: String,
            pub task_id: String,
            pub streaming: Streaming,
        }

        impl Header {
            pub fn new(task_id: &str) -> Self {
                Self {
                    action: "finish-task".to_string(),
                    task_id: task_id.to_string(),
                    streaming: Streaming::Duplex,
                }
            }
        }

        #[derive(Debug, serde::Serialize)]
        pub struct Request {
            header: Header,
            payload: Payload,
        }

        impl Request {
            #[must_use]
            pub fn new(task_id: &str) -> Self {
                Self {
                    header: Header::new(task_id),
                    payload: Payload::default(),
                }
            }
        }
    }

    pub mod response {
        use super::super::EmptyObj;
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct Payload {
            output: EmptyObj,
            usage: Option<EmptyObj>,
        }

        #[derive(Debug, Deserialize)]
        enum Event {
            #[serde(rename = "task-finished")]
            TaskFinished,
        }

        #[derive(Debug, Deserialize)]
        struct Header {
            task_id: String,
            event: Event,
            attributes: EmptyObj,
        }

        #[derive(Debug, Deserialize)]
        pub struct Response {
            header: Header,
            payload: Payload,
        }
    }
}

#[allow(dead_code)]
pub mod task_failed {
    use super::EmptyObj;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    enum Event {
        #[serde(rename = "task-failed")]
        TaskFailed,
    }

    #[derive(Debug, Deserialize)]
    struct Header {
        task_id: String,
        event: Event,
        pub error_code: String,
        pub error_message: String,
        attributes: EmptyObj,
    }

    #[derive(Debug, Deserialize)]
    pub struct Response {
        header: Header,
        payload: EmptyObj,
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ServerEvent {
    TaskStarted(run_task::response::Response),
    ResultGenerated(result_generated::Response),
    TaskFinished(finish_task::response::Response),
    TaskFailed(task_failed::Response),
}

impl From<result_generated::Response> for TranscribeResponse {
    fn from(value: result_generated::Response) -> Self {
        Self {
            text: value.payload.output.sentence.text,
            begin_time: value.payload.output.sentence.begin_time,
            sentence_end: value.payload.output.sentence.sentence_end,
        }
    }
}
