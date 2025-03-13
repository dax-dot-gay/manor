use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Deserialization failed: {0:?}")]
    Deserialization(bson::de::Error),

    #[error("Serialization failed: {0:?}")]
    Serialization(bson::ser::Error),

    #[error("Invalid connection URI ({0}): {1:?}")]
    InvalidUri(String, mongodb::error::Error),

    #[error("Failed to create MongoDB client: {0:?}")]
    ClientFailure(mongodb::error::Error),

    #[error("Mongodb operation failed: {0:?}")]
    MongoError(mongodb::error::Error),

    #[error("Queried document not found.")]
    NotFound
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

impl From<mongodb::error::Error> for Error {
    fn from(value: mongodb::error::Error) -> Self {
        Self::MongoError(value)
    }
}

pub type MResult<T> = Result<T, Error>;
