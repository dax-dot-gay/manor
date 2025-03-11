use std::fmt::Debug;

use bson::oid::ObjectId;
use serde::{de::DeserializeOwned, Serialize};

use crate::collection::Collection;

pub trait Schema: Serialize + DeserializeOwned + Send + Sync + Clone + Debug {
    fn id_field() -> String;
    fn collection_name() -> String;
    fn id(&self) -> ObjectId;
}

#[derive(Clone, Debug)]
pub struct Model<S: Schema> {
    data: S,
    collection: Collection<S>
}

impl<S: Schema> Model<S> {
    pub(crate) fn _create(data: S, collection: Collection<S>) -> Self {
        Self {
            data, collection
        }
    }
}