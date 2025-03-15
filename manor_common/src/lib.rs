pub mod model;
pub mod collection;
pub mod error;
pub mod client;
pub mod types;
pub mod gridfs;

/// Global instance of the Client, set using
/// 
/// ```
/// Client::connect_with_*().as_global();
/// ```
pub use client::MANOR_CLIENT;

#[doc(hidden)]
pub use {
    serde, bson, uuid, derive_builder
};