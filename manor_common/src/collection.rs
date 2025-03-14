use std::task::Poll;

use bson::{Bson, Document, doc, from_bson};
use futures_core::Stream;
use mongodb::{
    Namespace,
    options::{
        AggregateOptions, CountOptions, DeleteOptions, EstimatedDocumentCountOptions,
        FindOneAndDeleteOptions, FindOneAndReplaceOptions, FindOneAndUpdateOptions, FindOneOptions,
        FindOptions, InsertManyOptions, InsertOneOptions, ReplaceOptions, UpdateModifications,
        UpdateOptions,
    },
    results::UpdateResult,
};

use crate::{
    client::Client,
    error::{Error, MResult},
    model::Model,
};

#[derive(Clone, Debug)]
pub struct Collection<M: Model + Send + Sync> {
    pub(crate) collection: mongodb::Collection<M>,
    pub(crate) client: Client,
}

#[derive(Clone, Debug)]
pub enum Ops {
    Many,
    One,
}

#[derive(Clone, Debug)]
pub enum Find<M: Model + Send + Sync> {
    Many(Option<FindOptions>),
    One(Option<FindOneOptions>),
    Delete(Option<FindOneAndDeleteOptions>),
    Replace {
        replacement: M,
        options: Option<FindOneAndReplaceOptions>,
        upsert: bool,
    },
    Update {
        modifications: UpdateModifications,
        options: Option<FindOneAndUpdateOptions>,
    },
}

pub enum FindResult<M: Model + Send + Sync> {
    Cursor(Cursor<M>),
    Single(Option<M>),
}

impl<M: Model + Send + Sync> FindResult<M> {
    pub fn cursor(self) -> Option<Cursor<M>> {
        if let Self::Cursor(c) = self {
            Some(c)
        } else {
            None
        }
    }

    pub fn single(self) -> Option<Option<M>> {
        if let Self::Single(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

impl<M: Model + Send + Sync> Find<M> {
    pub fn many() -> Self {
        Self::Many(None)
    }

    pub fn one() -> Self {
        Self::One(None)
    }

    pub fn delete() -> Self {
        Self::Delete(None)
    }

    pub fn replace(replacement: M) -> Self {
        Self::Replace {
            replacement,
            options: None,
            upsert: false,
        }
    }

    pub fn replace_or_insert(replacement: M) -> Self {
        Self::Replace {
            replacement,
            options: None,
            upsert: true,
        }
    }

    pub fn update(modifications: impl Into<UpdateModifications>) -> Self {
        Self::Update {
            modifications: modifications.into(),
            options: None,
        }
    }
}

#[pin_project::pin_project]
pub struct Cursor<M: Model + Send + Sync> {
    pub(crate) collection: Collection<M>,

    #[pin]
    pub(crate) base: mongodb::Cursor<M>,
}

impl<M: Model + Send + Sync> Stream for Cursor<M> {
    type Item = MResult<M>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let projected = self.project();
        let base = projected.base;
        match base.poll_next(cx) {
            Poll::Ready(Some(Ok(record))) => {
                let mut rec = record.clone();
                rec.attach_collection(projected.collection.clone());
                Poll::Ready(Some(Ok(rec)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<M: Model + Send + Sync> Collection<M> {
    fn parse_id(bson: &Bson) -> Option<M::Id> {
        if let Ok(parsed) = from_bson::<M::Id>(bson.clone()) {
            Some(parsed)
        } else {
            None
        }
    }

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

    pub fn wrap(&self, mut model: M) -> M {
        model.attach_collection(self.clone());
        model
    }

    pub fn cursor(&self, cursor: mongodb::Cursor<M>) -> Cursor<M> {
        Cursor::<M> {
            collection: self.clone(),
            base: cursor,
        }
    }

    pub async fn aggregate_with_options<T>(
        &self,
        pipeline: impl IntoIterator<Item = Document>,
        options: impl Into<Option<AggregateOptions>>,
    ) -> MResult<mongodb::Cursor<T>> {
        self.collection()
            .aggregate(pipeline)
            .with_type::<T>()
            .with_options(options)
            .await
            .or_else(|e| Err(e.into()))
    }

    pub async fn aggregate(
        &self,
        pipeline: impl IntoIterator<Item = Document>,
    ) -> MResult<mongodb::Cursor<Document>> {
        self.aggregate_with_options::<Document>(pipeline, None)
            .await
    }

    pub async fn aggregate_typed<T>(
        &self,
        pipeline: impl IntoIterator<Item = Document>,
    ) -> MResult<mongodb::Cursor<T>> {
        self.aggregate_with_options::<T>(pipeline, None).await
    }

    pub async fn exact_count_with_options(
        &self,
        query: impl Into<Document>,
        options: impl Into<Option<CountOptions>>,
    ) -> MResult<u64> {
        self.collection()
            .count_documents(query.into())
            .with_options(options)
            .await
            .or_else(|e| Err(e.into()))
    }

    pub async fn estimated_count_with_options(
        &self,
        options: impl Into<Option<EstimatedDocumentCountOptions>>,
    ) -> MResult<u64> {
        self.collection()
            .estimated_document_count()
            .with_options(options)
            .await
            .or_else(|e| Err(e.into()))
    }

    pub async fn exact_count(&self, query: impl Into<Document>) -> MResult<u64> {
        self.exact_count_with_options(query, None).await
    }

    pub async fn estimated_count(&self) -> MResult<u64> {
        self.estimated_count_with_options(None).await
    }

    pub async fn delete_with_options(
        &self,
        query: impl Into<Document>,
        operations: Ops,
        options: impl Into<Option<DeleteOptions>>,
    ) -> MResult<u64> {
        let collection = self.collection();
        match operations {
            Ops::Many => collection.delete_many(query.into()).with_options(options),
            Ops::One => collection.delete_one(query.into()).with_options(options),
        }
        .await
        .or_else(|e| Err(e.into()))
        .and_then(|v| Ok(v.deleted_count))
    }

    pub async fn delete_one(&self, query: impl Into<Document>) -> MResult<()> {
        match self.delete_with_options(query, Ops::One, None).await {
            Ok(1) => Ok(()),
            Ok(_) => Err(Error::NotFound),
            Err(e) => Err(e),
        }
    }

    pub async fn delete_many(&self, query: impl Into<Document>) -> MResult<u64> {
        self.delete_with_options(query, Ops::Many, None).await
    }

    pub async fn find(&self, query: impl Into<Document>, find: Find<M>) -> MResult<FindResult<M>> {
        let collection = self.collection();
        match find {
            Find::Many(options) => collection
                .find(query.into())
                .with_options(options)
                .await
                .and_then(|c| Ok(FindResult::Cursor(self.cursor(c))))
                .or_else(|e| Err(e.into())),
            Find::One(options) => collection
                .find_one(query.into())
                .with_options(options)
                .await
                .and_then(|r| Ok(FindResult::Single(r)))
                .or_else(|e| Err(e.into())),
            Find::Delete(options) => collection
                .find_one_and_delete(query.into())
                .with_options(options)
                .await
                .and_then(|r| Ok(FindResult::Single(r)))
                .or_else(|e| Err(e.into())),
            Find::Replace {
                replacement,
                options,
                upsert,
            } => collection
                .find_one_and_replace(query.into(), replacement)
                .with_options(options)
                .upsert(upsert)
                .await
                .and_then(|r| Ok(FindResult::Single(r)))
                .or_else(|e| Err(e.into())),
            Find::Update {
                modifications,
                options,
            } => collection
                .find_one_and_update(query.into(), modifications)
                .with_options(options)
                .await
                .and_then(|r| Ok(FindResult::Single(r)))
                .or_else(|e| Err(e.into())),
        }
    }

    pub async fn find_many(&self, query: impl Into<Document>) -> MResult<Cursor<M>> {
        self.find(query, Find::<M>::many())
            .await
            .and_then(|r| Ok(r.cursor().unwrap()))
    }

    pub async fn find_one(&self, query: impl Into<Document>) -> MResult<Option<M>> {
        self.find(query, Find::<M>::one())
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    pub async fn find_one_and_delete(&self, query: impl Into<Document>) -> MResult<Option<M>> {
        self.find(query, Find::<M>::delete())
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    pub async fn find_one_and_replace(
        &self,
        query: impl Into<Document>,
        replacement: M,
    ) -> MResult<Option<M>> {
        self.find(query, Find::<M>::replace(replacement))
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    pub async fn find_one_and_upsert(
        &self,
        query: impl Into<Document>,
        replacement: M,
    ) -> MResult<Option<M>> {
        self.find(query, Find::<M>::replace_or_insert(replacement))
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    pub async fn find_one_and_update(
        &self,
        query: impl Into<Document>,
        update: impl Into<UpdateModifications>,
    ) -> MResult<Option<M>> {
        self.find(query, Find::<M>::update(update))
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    pub async fn insert_many_with_options(
        &self,
        documents: impl IntoIterator<Item = M>,
        options: impl Into<Option<InsertManyOptions>>,
    ) -> MResult<Vec<M::Id>> {
        self.collection()
            .insert_many(documents)
            .with_options(options)
            .await
            .and_then(|r| Ok(r.inserted_ids.values().filter_map(Self::parse_id).collect()))
            .or_else(|e| Err(e.into()))
    }

    pub async fn insert_one_with_options(
        &self,
        document: M,
        options: impl Into<Option<InsertOneOptions>>,
    ) -> MResult<Option<M::Id>> {
        self.collection()
            .insert_one(document)
            .with_options(options)
            .await
            .and_then(|r| Ok(Self::parse_id(&r.inserted_id)))
            .or_else(|e| Err(e.into()))
    }

    pub async fn insert_many(&self, documents: impl IntoIterator<Item = M>) -> MResult<Vec<M::Id>> {
        self.insert_many_with_options(documents, None).await
    }

    pub async fn insert_one(&self, document: M) -> MResult<Option<M::Id>> {
        self.insert_one_with_options(document, None).await
    }

    pub async fn replace_one_with_options(
        &self,
        query: impl Into<Document>,
        document: M,
        upsert: bool,
        options: impl Into<Option<ReplaceOptions>>,
    ) -> MResult<Option<M::Id>> {
        self.collection()
            .replace_one(query.into(), document)
            .with_options(options)
            .upsert(upsert)
            .await
            .and_then(|r| Ok(r.upserted_id.and_then(|i| Self::parse_id(&i))))
            .or_else(|e| Err(e.into()))
    }

    pub async fn replace_one(
        &self,
        query: impl Into<Document>,
        document: M,
    ) -> MResult<Option<M::Id>> {
        self.replace_one_with_options(query, document, false, None)
            .await
    }

    pub async fn replace_or_insert_one(
        &self,
        query: impl Into<Document>,
        document: M,
    ) -> MResult<Option<M::Id>> {
        self.replace_one_with_options(query, document, true, None)
            .await
    }

    pub async fn update_with_options(
        &self,
        query: impl Into<Document>,
        update: impl Into<UpdateModifications>,
        operations: Ops,
        options: impl Into<Option<UpdateOptions>>,
    ) -> MResult<UpdateResult> {
        let collection = self.collection();
        match operations {
            Ops::One => collection
                .update_one(query.into(), update)
                .with_options(options)
                .await
                .or_else(|e| Err(e.into())),
            Ops::Many => collection
                .update_many(query.into(), update)
                .with_options(options)
                .await
                .or_else(|e| Err(e.into())),
        }
    }

    pub async fn update_one(
        &self,
        query: impl Into<Document>,
        update: impl Into<UpdateModifications>,
    ) -> MResult<UpdateResult> {
        self.update_with_options(query, update, Ops::One, None)
            .await
    }

    pub async fn update_many(
        &self,
        query: impl Into<Document>,
        update: impl Into<UpdateModifications>,
    ) -> MResult<UpdateResult> {
        self.update_with_options(query, update, Ops::Many, None)
            .await
    }

    pub fn name(&self) -> String {
        self.collection().name().to_string()
    }

    pub fn namespace(&self) -> Namespace {
        self.collection().namespace().clone()
    }

    pub async fn get(&self, id: impl Into<M::Id>) -> MResult<Option<M>> {
        self.find_one(doc! {"_id": Into::<M::Id>::into(id)}).await
    }

    pub async fn save(&self, document: M) -> MResult<()> {
        self.replace_or_insert_one(doc! {"_id": document.id()}, document)
            .await
            .and(Ok(()))
    }
}
