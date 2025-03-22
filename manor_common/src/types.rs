use serde::{Deserialize, Serialize};

use crate::{
    MANOR_CLIENT,
    client::Client,
    error::{Error, MResult},
    model::Model,
};

/// A Link struct, representing a document in another collection.
/// The referenced model must implement [Model]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link<M: Model + Send + Sync> {
    /// Name of the collection. Not used directly, but useful for external parsing.
    pub collection: String,

    /// The ID of the targeted document
    pub id: M::Id,
    #[serde(skip, default)]
    resolved: Option<M>,

    #[serde(skip, default)]
    client: Option<Client>,
}

impl<M: Model + Send + Sync> Link<M> {
    /// Gets either the local or global client (in that order of precedence). Panics if no client has been initialized.
    pub fn client(&self) -> Client {
        self.client.clone().unwrap_or(
            MANOR_CLIENT
                .get()
                .expect("This Link<> has no connection to a client.")
                .clone(),
        )
    }

    /// Attaches a [Client] to this [Link]
    pub fn with_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Resolves the referenced value and returns it. If the value has been already retrieved, just returns it directly.
    pub async fn resolve(&mut self) -> MResult<M> {
        if let Some(val) = self.resolved.clone() {
            Ok(val)
        } else {
            let result = self.client().collection::<M>().get(self.id.clone()).await?;
            if let Some(found) = result {
                self.resolved = Some(found.clone());
                Ok(found)
            } else {
                Err(Error::NotFound)
            }
        }
    }

    /// Forces the contained value to refresh (unless the document has been deleted in the meantime) and returns it.
    pub async fn refresh(&mut self) -> MResult<M> {
        let result = self.client().collection::<M>().get(self.id.clone()).await?;
        if let Some(found) = result {
            self.resolved = Some(found.clone());
            Ok(found)
        } else {
            Err(Error::NotFound)
        }
    }

    /// Gets a reference to the contained value, if resolved
    pub fn value(&self) -> Option<&M> {
        self.resolved.as_ref()
    }

    /// Gets a mutable reference to the contained value, if resolved
    pub fn value_mut(&mut self) -> Option<&mut M> {
        self.resolved.as_mut()
    }
}

impl<M: Model + Send + Sync> From<M> for Link<M> {
    fn from(value: M) -> Self {
        Self {
            collection: M::collection_name(),
            id: value.id(),
            resolved: Some(value),
            client: None,
        }
    }
}
