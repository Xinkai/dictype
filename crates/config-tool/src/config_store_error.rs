#[derive(thiserror::Error, Debug)]
pub enum ConfigStoreError {
    #[error("missing HOME directory")]
    MissingHome,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("toml write error: {0}")]
    TomlWrite(#[from] toml::ser::Error),
}
