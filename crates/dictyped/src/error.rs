#[derive(thiserror::Error, Debug)]
pub enum DictypeError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<DictypeError> for tonic::Status {
    fn from(e: DictypeError) -> Self {
        Self::internal(format!("{e:?}"))
    }
}
