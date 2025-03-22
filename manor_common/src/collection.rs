use std::task::Poll;

use bson::{doc, from_bson, to_document, Bson, Document};
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

/// A wrapper around [mongodb::Collection] with abstractions for common operations
#[derive(Clone, Debug)]
pub struct Collection<M: Model + Send + Sync> {
    pub(crate) collection: mongodb::Collection<M>,
    pub(crate) client: Client,
}

/// An enum describing how many operations to run, in certain cases
#[derive(Clone, Debug)]
pub enum Ops {
    /// Run many operations (ie find_many, delete_many, etc)
    Many,

    /// Run at most one operation (ie find_one, delete_one, etc)
    One,
}

/// A wrapper for the options of several `find` operations. Should not need to be manually constructed in most cases.
#[derive(Clone, Debug)]
pub enum Find<M: Model + Send + Sync> {
    /// Used for [mongodb::Collection::find]
    Many(Option<FindOptions>),

    /// Used for [mongodb::Collection::find_one]
    One(Option<FindOneOptions>),

    /// Used for [mongodb::Collection::find_one_and_delete]
    Delete(Option<FindOneAndDeleteOptions>),

    /// Used for [mongodb::Collection::find_one_and_replace]
    Replace {
        /// Document to replace with
        replacement: M,

        /// Find/replace options
        options: Option<FindOneAndReplaceOptions>,

        /// Whether to upsert (insert if no document was found)
        upsert: bool,
    },

    /// Used for [mongodb::Collection::find_one_and_update]
    Update {
        /// Update modifications to apply
        modifications: UpdateModifications,

        /// Find/update options
        options: Option<FindOneAndUpdateOptions>,
    },
}

/// The result of a Find operation
pub enum FindResult<M: Model + Send + Sync> {
    /// A cursor over multiple documents
    Cursor(Cursor<M>),

    /// A single document, or [None]
    Single(Option<M>),
}

impl<M: Model + Send + Sync> FindResult<M> {
    /// Returns a [Cursor] that wraps [mongodb::Cursor], if this result was of type [FindResult::Cursor]
    pub fn cursor(self) -> Option<Cursor<M>> {
        if let Self::Cursor(c) = self {
            Some(c)
        } else {
            None
        }
    }

    /// Returns [`Option<M>`] if this result was of type [FindResult::Single]
    pub fn single(self) -> Option<Option<M>> {
        if let Self::Single(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

#[allow(missing_docs)]
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

/// A wrapper around [mongodb::Cursor] that attaches the current collection to results automatically
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

    /// Gets this collection's [Client]
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    /// Gets a collection from a local [Client]
    pub fn new_local(client: Client) -> Self {
        client.collection::<M>()
    }

    /// Gets a collection from the global [Client], if present
    pub fn new_global() -> Option<Self> {
        Client::global().and_then(|c| Some(c.collection::<M>()))
    }

    /// Gets a collection from the global [Client], panicking if the global [Client] is undefined.
    pub fn new() -> Self {
        Client::global()
            .expect("Global client not initialized.")
            .collection::<M>()
    }

    /// Returns the underlying [mongodb::Collection]
    pub fn collection(&self) -> mongodb::Collection<M> {
        self.collection.clone()
    }

    /// Attaches this collection to a [Model]
    pub fn wrap(&self, mut model: M) -> M {
        model.attach_collection(self.clone());
        model
    }

    /// Wraps a [mongodb::Cursor] in a [Cursor]
    pub fn cursor(&self, cursor: mongodb::Cursor<M>) -> Cursor<M> {
        Cursor::<M> {
            collection: self.clone(),
            base: cursor,
        }
    }

    /// Runs aggregation with a defined type & options
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

    /// Runs a simple untyped aggregation
    pub async fn aggregate(
        &self,
        pipeline: impl IntoIterator<Item = Document>,
    ) -> MResult<mongodb::Cursor<Document>> {
        self.aggregate_with_options::<Document>(pipeline, None)
            .await
    }

    /// Runs a simple typed aggregation
    pub async fn aggregate_typed<T>(
        &self,
        pipeline: impl IntoIterator<Item = Document>,
    ) -> MResult<mongodb::Cursor<T>> {
        self.aggregate_with_options::<T>(pipeline, None).await
    }

    /// Gets an exact document count with options
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

    /// Gets an estimated document count with options
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

    /// Default exact_count
    pub async fn exact_count(&self, query: impl Into<Document>) -> MResult<u64> {
        self.exact_count_with_options(query, None).await
    }

    /// Default estimated_count
    pub async fn estimated_count(&self) -> MResult<u64> {
        self.estimated_count_with_options(None).await
    }

    /// Deletes [Ops::One] or [Ops::Many] documents with options
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

    /// Deletes one document
    pub async fn delete_one(&self, query: impl Into<Document>) -> MResult<()> {
        match self.delete_with_options(query, Ops::One, None).await {
            Ok(1) => Ok(()),
            Ok(_) => Err(Error::NotFound),
            Err(e) => Err(e),
        }
    }

    /// Deletes all documents matching a query
    pub async fn delete_many(&self, query: impl Into<Document>) -> MResult<u64> {
        self.delete_with_options(query, Ops::Many, None).await
    }

    /// Performs an advanced Find operation
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

    /// Finds many documents, returning an iterable [Cursor]
    pub async fn find_many(&self, query: impl Into<Document>) -> MResult<Cursor<M>> {
        self.find(query, Find::<M>::many())
            .await
            .and_then(|r| Ok(r.cursor().unwrap()))
    }

    /// Finds at most one document
    pub async fn find_one(&self, query: impl Into<Document>) -> MResult<Option<M>> {
        self.find(query, Find::<M>::one())
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    /// Finds one document, then deletes it.
    pub async fn find_one_and_delete(&self, query: impl Into<Document>) -> MResult<Option<M>> {
        self.find(query, Find::<M>::delete())
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    /// Finds one document, then replaces it
    pub async fn find_one_and_replace(
        &self,
        query: impl Into<Document>,
        replacement: M,
    ) -> MResult<Option<M>> {
        self.find(query, Find::<M>::replace(replacement))
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    /// Finds one document, upserting if not found and replacing otherwise
    pub async fn find_one_and_upsert(
        &self,
        query: impl Into<Document>,
        replacement: M,
    ) -> MResult<Option<M>> {
        self.find(query, Find::<M>::replace_or_insert(replacement))
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    /// Finds one document and updates it
    pub async fn find_one_and_update(
        &self,
        query: impl Into<Document>,
        update: impl Into<UpdateModifications>,
    ) -> MResult<Option<M>> {
        self.find(query, Find::<M>::update(update))
            .await
            .and_then(|r| Ok(r.single().unwrap()))
    }

    /// Inserts many documents with options
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

    /// Inserts one document with options
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

    /// Simplified insert_many
    pub async fn insert_many(&self, documents: impl IntoIterator<Item = M>) -> MResult<Vec<M::Id>> {
        self.insert_many_with_options(documents, None).await
    }

    /// Simplified insert_one
    pub async fn insert_one(&self, document: M) -> MResult<Option<M::Id>> {
        self.insert_one_with_options(document, None).await
    }

    /// Replaces a document, with options. Optionally upserts.
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

    /// Replaces a document without upserting
    pub async fn replace_one(
        &self,
        query: impl Into<Document>,
        document: M,
    ) -> MResult<Option<M::Id>> {
        self.replace_one_with_options(query, document, false, None)
            .await
    }

    /// Replaces a document, or inserts it if not present
    pub async fn replace_or_insert_one(
        &self,
        query: impl Into<Document>,
        document: M,
    ) -> MResult<Option<M::Id>> {
        let _query: Document = query.into();
        if let Ok(Some(_)) = self.collection.find_one(_query.clone()).await {
            let mut as_doc = to_document(&document).or_else(|e| Err(Error::Serialization(e)))?;
            let _ = as_doc.remove("_id");
            self.client().database().collection::<Document>(&M::collection_name()).replace_one(_query, as_doc).await.or_else(|e| Err(Error::MongoError(e)))?;
            Ok(Some(document.id()))
        } else {
            self.insert_one(document.clone()).await?;
            Ok(Some(document.id()))
        }
    }

    /// Updates [Ops::One] or [Ops::Many] documents, with options
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

    /// Updates a single document
    pub async fn update_one(
        &self,
        query: impl Into<Document>,
        update: impl Into<UpdateModifications>,
    ) -> MResult<UpdateResult> {
        self.update_with_options(query, update, Ops::One, None)
            .await
    }

    /// Updates many documents
    pub async fn update_many(
        &self,
        query: impl Into<Document>,
        update: impl Into<UpdateModifications>,
    ) -> MResult<UpdateResult> {
        self.update_with_options(query, update, Ops::Many, None)
            .await
    }

    /// Gets the name of this collection
    pub fn name(&self) -> String {
        self.collection().name().to_string()
    }

    /// Gets the namespace (database.collection) of this collection
    pub fn namespace(&self) -> Namespace {
        self.collection().namespace().clone()
    }

    /// Gets a document by ID
    pub async fn get(&self, id: impl Into<M::Id>) -> MResult<Option<M>> {
        self.find_one(doc! {"_id": Into::<M::Id>::into(id)}).await
    }

    /// Helper function to save a document (insert or replace by ID)
    pub async fn save(&self, document: M) -> MResult<()> {
        self.replace_or_insert_one(doc! {"_id": document.id()}, document)
            .await
            .and(Ok(()))
    }

    /// Helper function to delete the passed document
    pub async fn delete(&self, document: M) -> MResult<()> {
        self.delete_one(doc! {"_id": document.id()}).await
    }
}
