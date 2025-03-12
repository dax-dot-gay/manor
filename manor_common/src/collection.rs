use std::task::Poll;

use futures_core::TryStream;
use futures_util::{Stream, StreamExt, TryStreamExt};
use mongodb::Cursor;
use pin_project::pin_project;

use crate::model::{Model, Schema};

#[derive(Clone, Debug)]
pub struct Collection<S: Schema>(mongodb::Collection<S>);

#[pin_project]
pub struct ManorCursor<S: Schema> {
    #[pin]
    cursor: Cursor<S>,
    collection: Collection<S>
}

impl<S: Schema> ManorCursor<S> {
    pub(crate) fn new(cursor: Cursor<S>, collection: Collection<S>) -> Self {
        Self {
            cursor, collection
        }
    }
}

impl<S: Schema> Stream for ManorCursor<S> {
    type Item = Result<Model<S>, mongodb::error::Error>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let mut referenced = self.project();
        let mut cursor = referenced.cursor.as_mut();
        match cursor.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(schema))) => Poll::Ready(Some(Ok(Model::_create(schema, referenced.collection.clone())))),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None)
        }
    }
}

impl<S: Schema> Collection<S> {
    fn wrap(&self, result: S) -> Model<S> {
        Model::<S>::_create(result, self.clone())
    }

    fn wrap_cursor(&self, cursor: Cursor<S>) -> ManorCursor<S> {
        ManorCursor::<S>::new(cursor, self.clone())
    }

    
}
