use std::task::Poll;

use futures_util::{Stream, StreamExt};
use mongodb::Cursor;

use crate::model::{Model, Schema};

#[derive(Clone, Debug)]
pub struct Collection<S: Schema>(mongodb::Collection<S>);

pub struct ManorCursor<S: Schema>(Cursor<S>);

impl<S: Schema> ManorCursor<S> {
    pub(crate) fn new(cursor: Cursor<S>) -> Self {
        Self(cursor)
    }
}

impl<S: Schema> Stream for ManorCursor<S> {
    type Item = Model<S>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match self.0.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(item))
        }
    }
}

impl<S: Schema> Collection<S> {
    fn wrap(&self, result: S) -> Model<S> {
        Model::<S>::_create(result, self.clone())
    }

    fn wrap_cursor(&self, cursor: Cursor<S>)
}
