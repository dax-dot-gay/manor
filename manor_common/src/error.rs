use thiserror::Error;

/// An enum describing possible Manor errors.
#[derive(Error, Debug)]
pub enum Error {
    /// BSON deserialization has failed
    #[error("Deserialization failed: {0:?}")]
    Deserialization(bson::de::Error),

    /// BSON serialization has failed
    #[error("Serialization failed: {0:?}")]
    Serialization(bson::ser::Error),

    /// The provided connection URI was invalid
    #[error("Invalid connection URI ({0}): {1:?}")]
    InvalidUri(String, mongodb::error::Error),

    /// Failed to create a client
    #[error("Failed to create MongoDB client: {0:?}")]
    ClientFailure(mongodb::error::Error),

    /// An internal MongoDB error occurred
    #[error("Mongodb operation failed: {0:?}")]
    MongoError(mongodb::error::Error),

    /// The requested record/item was not found
    #[error("Queried document not found.")]
    NotFound,

    /// The [crate::types::Link] was unable to be resolved
    #[error("The linked document has not been resolved yet: {0}::{1}")]
    UnresolvedLink(String, String),

    /// A write operation failed
    #[error("Failed to write data to GridFS")]
    WriteFailure(String)
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

/// Utility type for functions returning [enum@Error]
pub type MResult<T> = Result<T, Error>;
