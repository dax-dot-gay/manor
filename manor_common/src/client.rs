use std::cell::OnceCell;

use crate::{collection::Collection, error::{Error, MResult}, model::Model};

pub const MANOR_CLIENT: OnceCell<Client> = OnceCell::new();

#[derive(Clone, Debug)]
pub struct Client {
    client: mongodb::Client,
    database: String
}

impl Client {
    pub fn database(&self) -> mongodb::Database {
        self.client.database(&self.database)
    }

    pub fn collection<M: Model + Send + Sync>(&self) -> Collection<M> {
        Collection { collection: self.database().collection(&M::collection_name()), client: self.clone() }
    }

    pub fn connect_with_uri(uri: impl Into<String>, database: impl Into<String>) -> MResult<Self> {
        let converted = uri.into();
        let connection_str = mongodb::options::ConnectionString::parse(&converted).or_else(|e| Err(Error::InvalidUri(converted.clone(), e)))?;
        let options = mongodb::options::ClientOptions::parse(connection_str).run().or_else(|e| Err(Error::InvalidUri(converted.clone(), e)))?;
        Self::connect_with_options(options, database)
    }

    pub fn connect_with_options(options: mongodb::options::ClientOptions, database: impl Into<String>) -> MResult<Self> {
        Ok(Self {
            client: mongodb::Client::with_options(options).or_else(|e| Err(Error::ClientFailure(e)))?,
            database: database.into()
        })
    }

    pub fn connect_with_client(client: mongodb::Client, database: impl Into<String>) -> Self {
        Self {
            client,
            database: database.into()
        }
    }

    pub fn as_global(self) {
        MANOR_CLIENT.set(self).expect("A global client was already set.");
    }

    pub fn global() -> Option<Self> {
        MANOR_CLIENT.get().cloned()
    }
}

impl From<mongodb::Database> for Client {
    fn from(value: mongodb::Database) -> Self {
        Self {
            client: value.client().clone(),
            database: value.name().to_string()
        }
    }
}