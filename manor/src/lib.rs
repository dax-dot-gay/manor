//! A highly-abstracted MongoDB ORM, with additional helpers for GridFS operations and document links.
//! 
//! Currently not production-ready, still very much in an early stage.

#[doc(inline)]
pub use manor_common::{
    collection::Collection,
    error::{Error, MResult},
    gridfs::{self, GridFS, GridFile},
    model::Model,
    types::Link,
    client::Client
};

#[doc(inline)]
pub use manor_macros::schema;

#[doc(hidden)]
pub use manor_common::{
    serde,
    uuid,
    bson,
    derive_builder
};
