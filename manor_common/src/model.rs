use std::{fmt::Debug, ops::{Deref, DerefMut}};

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

impl<S: Schema> Deref for Model<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<S: Schema> DerefMut for Model<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}