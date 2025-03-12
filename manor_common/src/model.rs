use std::fmt::Debug;

use bson::oid::ObjectId;
use serde::{de::DeserializeOwned, Serialize};

use crate::{collection::Collection, error::MResult, MANOR_CLIENT};

#[async_trait::async_trait]
pub trait Model: Serialize + DeserializeOwned + Clone + Debug + Send + Sync {
    fn from_document(document: bson::Document, collection: Option<Collection<Self>>) -> MResult<Self>;
    fn collection_name() -> String;
    fn own_collection(&self) -> Option<Collection<Self>>;
    fn id(&self) -> ObjectId;
    fn attach_collection(&mut self, collection: Collection<Self>) -> ();

    fn collection(&self) -> Collection<Self> {
        if let Some(coll) = self.own_collection() {
            coll
        } else {
            MANOR_CLIENT.get().expect("Neither a local nor global client has been initialized.").clone().collection::<Self>()
        }
    }
}