use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Deserialization failed: {0:?}")]
    Deserialization(bson::de::Error),

    #[error("Serialization failed: {0:?}")]
    Serialization(bson::ser::Error),
}

impl From<bson::de::Error> for Error {
    fn from(value: bson::de::Error) -> Self {
        Self::Deserialization(value)
    }
}

impl From<bson::ser::Error> for Error {
    fn from(value: bson::ser::Error) -> Self {
        Self::Serialization(value)
    }
}

pub type MResult<T> = Result<T, Error>;
