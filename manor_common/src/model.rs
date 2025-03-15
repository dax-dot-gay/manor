use std::fmt::Debug;
use bson::Bson;
use serde::{de::DeserializeOwned, Serialize};

use crate::{collection::Collection, error::MResult, MANOR_CLIENT};

/// A model trait. Likely should not be directly implemented, but instead generated with the `#[schema(...)]` attribute.
#[async_trait::async_trait]
pub trait Model: Serialize + DeserializeOwned + Clone + Debug + Send + Sync {
    /// The type of this Model's `_id` field. A [bson::oid::ObjectId], [uuid::Uuid], or [String] are probably the best choices here, and [bson::oid::ObjectId] is the macro default.
    type Id: DeserializeOwned + Serialize + Clone + Debug + Send + Sync + Into<Bson>;

    /// Parses a model from a [bson::Document], attaching the provided collection.
    fn from_document(document: bson::Document, collection: Option<Collection<Self>>) -> MResult<Self>;

    /// Returns the collection name
    fn collection_name() -> String;

    /// Returns the local collection, if present
    fn own_collection(&self) -> Option<Collection<Self>>;

    /// Returns this document's ID
    fn id(&self) -> Self::Id;

    /// Generates a new instance of this Model's ID type
    fn generate_id() -> Self::Id;

    /// Sets the local collection
    fn attach_collection(&mut self, collection: Collection<Self>) -> ();

    /// Gets the local collection if present, otherwise attempts to use the global client. Panics if neither is defined.
    fn collection(&self) -> Collection<Self> {
        if let Some(coll) = self.own_collection() {
            coll
        } else {
            MANOR_CLIENT.get().expect("Neither a local nor global client has been initialized.").clone().collection::<Self>()
        }
    }

    /// Utility function to update/save this record in the database
    async fn save(&self) -> MResult<()> {
        self.collection().save(self.clone()).await
    }

    /// Utility function to delete this record from the database. Drops the Model instance.
    async fn delete(self) -> MResult<()> {
        self.collection().delete(self).await
    }
}