use crate::model::Model;

#[derive(Clone, Debug)]
pub struct Collection<M: Model + Send + Sync>(mongodb::Collection<M>);
