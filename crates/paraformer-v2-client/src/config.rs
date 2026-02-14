use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParaformerV2Config {
    pub dashscope_api_key: String,
    pub language_hints: Vec<LanguageHint>,
}

impl ParaformerV2Config {
    pub fn from_parts(
        dashscope_api_key: impl Into<String>,
        language_hints: Vec<LanguageHint>,
    ) -> Self {
        Self {
            dashscope_api_key: dashscope_api_key.into(),
            language_hints,
        }
    }

    pub fn from_ui(dashscope_api_key: impl Into<String>, language_hints_csv: &str) -> Self {
        Self {
            dashscope_api_key: dashscope_api_key.into(),
            language_hints: parse_language_hints_csv_lenient(language_hints_csv),
        }
    }

    pub fn parse_language_hints_csv(input: &str) -> Result<Vec<LanguageHint>, Vec<String>> {
        let mut valid = Vec::new();
        let mut invalid = Vec::new();
        let mut seen = std::collections::BTreeSet::new();

        for entry in input.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }

            let normalized = entry.to_ascii_lowercase().replace('_', "-");
            match normalized.parse::<LanguageHint>() {
                Ok(hint) => {
                    if seen.insert(hint) {
                        valid.push(hint);
                    } else {
                        invalid.push(entry.to_string());
                    }
                }
                Err(()) => invalid.push(entry.to_string()),
            }
        }

        if invalid.is_empty() {
            Ok(valid)
        } else {
            Err(invalid)
        }
    }

    #[must_use]
    pub fn language_hints_csv(&self) -> String {
        self.language_hints
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn parse_language_hints_csv_lenient(input: &str) -> Vec<LanguageHint> {
    let mut valid = Vec::new();

    for entry in input.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let normalized = entry.to_ascii_lowercase().replace('_', "-");
        if let Ok(hint) = normalized.parse::<LanguageHint>() {
            valid.push(hint);
        }
    }

    valid
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LanguageHint {
    #[serde(rename = "zh")]
    Zh,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "yue")]
    Yue,
    #[serde(rename = "ko")]
    Ko,
    #[serde(rename = "de")]
    De,
    #[serde(rename = "fr")]
    Fr,
    #[serde(rename = "ru")]
    Ru,
}

impl std::fmt::Display for LanguageHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Self::Zh => "zh",
            Self::En => "en",
            Self::Ja => "ja",
            Self::Yue => "yue",
            Self::Ko => "ko",
            Self::De => "de",
            Self::Fr => "fr",
            Self::Ru => "ru",
        };
        f.write_str(code)
    }
}

impl std::str::FromStr for LanguageHint {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "zh" => Ok(Self::Zh),
            "en" => Ok(Self::En),
            "ja" => Ok(Self::Ja),
            "yue" => Ok(Self::Yue),
            "ko" => Ok(Self::Ko),
            "de" => Ok(Self::De),
            "fr" => Ok(Self::Fr),
            "ru" => Ok(Self::Ru),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LanguageHint, ParaformerV2Config, parse_language_hints_csv_lenient};

    #[test]
    fn rejects_locale_style_hints() {
        let hints = parse_language_hints_csv_lenient("en-US, zh-CN , ,ja-JP");
        assert!(hints.is_empty());
    }

    #[test]
    fn parses_valid_language_hints() {
        let hints = parse_language_hints_csv_lenient("en, zh, ja");
        assert_eq!(hints.len(), 3);
    }

    #[test]
    fn reports_invalid_language_hints() {
        let invalid =
            ParaformerV2Config::parse_language_hints_csv("en, xx, zh-CN, foo").unwrap_err();
        assert_eq!(
            invalid,
            vec!["xx".to_string(), "zh-CN".to_string(), "foo".to_string()]
        );
    }

    #[test]
    fn rejects_duplicate_language_hints() {
        let invalid = ParaformerV2Config::parse_language_hints_csv("ru,   ru").unwrap_err();
        assert_eq!(invalid, vec!["ru".to_string()]);
    }
}
