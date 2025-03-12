use std::task::Poll;

use bson::Document;
use futures_core::Stream;
use serde::{de::DeserializeOwned, Serialize};

use crate::{client::Client, error::{Error, MResult}, model::Model};

#[derive(Clone, Debug)]
pub struct Collection<M: Model + Send + Sync> {
    pub(crate) collection: mongodb::Collection<M>,
    pub(crate) client: Client
}

#[pin_project::pin_project]
pub struct Cursor<M: Model + Send + Sync> {
    collection: Collection<M>,

    #[pin]
    base: mongodb::Cursor<M>
}

impl<M: Model + Send + Sync> Stream for Cursor<M> {
    type Item = MResult<M>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let projected = self.project();
        let base = projected.base;
        match base.poll_next(cx) {
            Poll::Ready(Some(Ok(record))) => {
                let mut rec = record.clone();
                rec.attach_collection(projected.collection.clone());
                Poll::Ready(Some(Ok(rec)))
            },
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending
        }
    }
}

impl<M: Model + Send + Sync> Collection<M> {
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub fn new(client: Client) -> Self {
        client.collection::<M>()
    }

    pub fn global() -> Option<Self> {
        Client::global().and_then(|c| Some(c.collection::<M>()))
    }

    pub fn collection(&self) -> mongodb::Collection<M> {
        self.collection.clone()
    }

    pub async fn aggregate<T: Serialize + DeserializeOwned>(&self, pipeline: impl IntoIterator<Item = Document>) -> MResult<mongodb::Cursor<T>> {
        self.collection().aggregate(pipeline).with_type::<T>().await.or_else(|e| Err(Error::from(e)))
    }

    pub async fn aggregate_untyped(&self, pipeline: impl IntoIterator<Item = Document>) -> MResult<mongodb::Cursor<Document>> {
        self.aggregate::<Document>(pipeline).await
    }
}
