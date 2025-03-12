use std::fmt::Debug;

use bson::oid::ObjectId;
use serde::{de::DeserializeOwned, Serialize};

use crate::{collection::Collection, error::MResult};

pub trait Model: Serialize + DeserializeOwned + Clone + Debug + Send + Sync {
    fn from_document(document: bson::Document, collection: Collection<Self>) -> MResult<Self>;
    fn collection_name() -> String;
    fn collection(&self) -> Collection<Self>;
    fn id(&self) -> ObjectId;
}