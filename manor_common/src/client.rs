use once_cell::sync::OnceCell;

use mongodb::options::GridFsBucketOptions;

use crate::{
    collection::Collection,
    error::{Error, MResult},
    gridfs::GridFS,
    model::Model,
};

/// Global instance of the [Client], stored in a [OnceCell]
pub(crate) static MANOR_CLIENT: OnceCell<Client> = OnceCell::new();

/// A Manor client instance, wrapping the MongoDB client and a single database name.
#[derive(Clone, Debug)]
pub struct Client {
    client: mongodb::Client,
    database: String,
}

impl Client {
    /// Returns the underlying Mongo database
    pub fn database(&self) -> mongodb::Database {
        self.client.database(&self.database)
    }

    /// Returns a typed [Collection] from a model type
    pub fn collection<M: Model + Send + Sync>(&self) -> Collection<M> {
        Collection {
            collection: self.database().collection(&M::collection_name()),
            client: self.clone(),
        }
    }

    /// Creates a client from a MongoDB connection string
    pub async fn connect_with_uri(uri: impl Into<String>, database: impl Into<String>) -> MResult<Self> {
        let converted = uri.into();
        let connection_str = mongodb::options::ConnectionString::parse(&converted)
            .or_else(|e| Err(Error::InvalidUri(converted.clone(), e)))?;
        let options = mongodb::options::ClientOptions::parse(connection_str)
            .await
            .or_else(|e| Err(Error::InvalidUri(converted.clone(), e)))?;
        Self::connect_with_options(options, database).await
    }

    /// Creates a client from MongoDB client options
    pub async fn connect_with_options(
        options: mongodb::options::ClientOptions,
        database: impl Into<String>,
    ) -> MResult<Self> {
        Ok(Self {
            client: mongodb::Client::with_options(options)
                .or_else(|e| Err(Error::ClientFailure(e)))?,
            database: database.into(),
        })
    }

    /// Creates a client from an existing MongoDB client instance
    pub async fn connect_with_client(client: mongodb::Client, database: impl Into<String>) -> Self {
        Self {
            client,
            database: database.into(),
        }
    }

    /// Makes this instance global. As the instance is a global [std::cell::OnceCell], this method will panic if a global client has already been set.
    pub fn as_global(self) {
        MANOR_CLIENT
            .set(self)
            .expect("A global client was already set.");
    }

    /// Returns the global instance, if initialized.
    pub fn global() -> Option<Self> {
        MANOR_CLIENT.get().cloned()
    }

    /// Returns a [GridFS] instance based on this [Client]
    pub fn grid_fs(&self) -> GridFS {
        GridFS {
            bucket: self.database().gridfs_bucket(
                GridFsBucketOptions::builder()
                    .bucket_name("default".to_string())
                    .build(),
            ),
            client: self.clone(),
            name: String::from("default"),
        }
    }

    /// Returns a [GridFS] instance with a custom name
    pub fn named_grid_fs(&self, name: impl Into<String>) -> GridFS {
        let sname: String = name.into();
        GridFS {
            bucket: self.database().gridfs_bucket(
                GridFsBucketOptions::builder()
                    .bucket_name(sname.clone())
                    .build(),
            ),
            client: self.clone(),
            name: sname,
        }
    }
}

/// Allows a [Client] to be constructed from a [mongodb::Database]
impl From<mongodb::Database> for Client {
    fn from(value: mongodb::Database) -> Self {
        Self {
            client: value.client().clone(),
            database: value.name().to_string(),
        }
    }
}
