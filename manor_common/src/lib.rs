#![warn(missing_docs)]

//! Sub-library containing the non-macro portions of Manor

/// Submodule containing the [model::Model] trait
pub mod model;

/// Submodule containing a wrapper around [mongodb::Collection]
pub mod collection;

/// Submodule containing error & result types
pub mod error;

/// Submodule containing the [client::Client] wrapper
pub mod client;

/// Submodule containing the [types::Link] type and associated methods
pub mod types;

/// Submodule containing GridFS-related operations
pub mod gridfs;

/// Global instance of the Client, set using
/// 
/// ```
/// Client::connect_with_*().as_global();
/// ```
pub(crate) use client::MANOR_CLIENT;

#[doc(hidden)]
pub use {
    serde, bson, uuid, derive_builder
};