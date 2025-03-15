use serde::{Deserialize, Serialize};

use crate::{
    MANOR_CLIENT,
    client::Client,
    error::{Error, MResult},
    model::Model,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link<M: Model + Send + Sync> {
    pub collection: String,
    pub id: M::Id,
    #[serde(skip, default)]
    resolved: Option<M>,

    #[serde(skip, default)]
    client: Option<Client>,
}

impl<M: Model + Send + Sync> Link<M> {
    pub fn client(&self) -> Client {
        self.client.clone().unwrap_or(
            MANOR_CLIENT
                .get()
                .expect("This Link<> has no connection to a client.")
                .clone(),
        )
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

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

    pub async fn refresh(&mut self) -> MResult<M> {
        let result = self.client().collection::<M>().get(self.id.clone()).await?;
        if let Some(found) = result {
            self.resolved = Some(found.clone());
            Ok(found)
        } else {
            Err(Error::NotFound)
        }
    }

    pub fn value(&self) -> Option<M> {
        self.resolved.clone()
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
