use bson::{doc, from_document, to_document, Document};
use chrono::Utc;
use futures_util::{AsyncRead, AsyncWrite, AsyncWriteExt};
use mongodb::gridfs::{GridFsBucket, GridFsDownloadStream, GridFsUploadStream};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    client::Client,
    error::{Error, MResult},
};

/// A wrapper for MongoDB's GridFS
#[derive(Clone, Debug)]
pub struct GridFS {
    pub(crate) bucket: GridFsBucket,
    pub(crate) client: Client,
    pub(crate) name: String,
}

impl GridFS {
    /// Returns the internal [GridFsBucket]
    pub fn bucket(&self) -> GridFsBucket {
        self.bucket.clone()
    }

    /// Returns the internal [Client]
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    /// Returns the name of the referenced bucket
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Creates a [GridWriter] for the specified filename, that will resolve into a filled out [GridFile] when [GridWriter::commit()] is called.
    pub async fn upload(&self, filename: impl Into<String>) -> MResult<GridWriter> {
        GridFile {
            id: Uuid::new_v4(),
            filename: filename.into(),
            details: None,
            fs: Some(self.clone()),
            metadata: None,
        }
        .write()
        .await
    }

    /// Creates a [GridWriter] with attached metadata and a filename, that will resolve into a filled out [GridFile] when [GridWriter::commit()] is called.
    pub async fn upload_with_metadata(&self, filename: impl Into<String>, metadata: impl Serialize + DeserializeOwned) -> MResult<GridWriter> {
        GridFile {
            id: Uuid::new_v4(),
            filename: filename.into(),
            details: None,
            fs: Some(self.clone()),
            metadata: Some(to_document(&metadata).or_else(|e| Err::<_, Error>(e.into()))?),
        }
        .write()
        .await
    }

    /// Fetches an existing [GridFile] in this bucket.
    pub async fn fetch(&self, id: impl AsRef<Uuid>) -> MResult<GridFile> {
        let info = self
            .bucket()
            .find_one(doc! {"_id": id.as_ref()})
            .await
            .or_else(|e| Err(<mongodb::error::Error as Into<Error>>::into(e)))?
            .ok_or(Error::NotFound)?;

        Ok(GridFile {
            id: id.as_ref().clone(),
            filename: info.filename.unwrap_or(id.as_ref().to_string()),
            details: Some(FileDetails {
                length: info.length.clone(),
                chunk_size_bytes: info.chunk_size_bytes.clone(),
                upload_date: info.upload_date.clone().to_chrono(),
            }),
            metadata: info.metadata,
            fs: Some(self.clone()),
        })
    }

    /// Deletes a file by ID
    pub async fn delete(&self, id: impl AsRef<Uuid>) -> MResult<()> {
        self.bucket()
            .delete(id.as_ref().into())
            .await
            .or_else(|e| Err(e.into()))
    }
}

/// Metadata about a file, that is only known after the file is created.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileDetails {
    /// File length
    pub length: u64,

    /// Chunk size
    pub chunk_size_bytes: u32,

    /// Date of upload
    pub upload_date: chrono::DateTime<Utc>,
}

/// A representation of a file in GridFS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridFile {
    /// The file's ID
    pub id: Uuid,

    /// The filename of this file
    pub filename: String,

    /// Resolved details about the file, if it's been uploaded already.
    pub details: Option<FileDetails>,

    /// Arbitrary metadata stored in the file's document
    pub metadata: Option<Document>,

    #[serde(skip)]
    pub(crate) fs: Option<GridFS>,
}

impl GridFile {
    /// Creates a [GridReader] to read this file.
    /// 
    /// <div class="warning">Panics: If the GridFS instance has not been attached.</div>
    pub async fn read(&self) -> MResult<GridReader> {
        let reader = self
            .fs
            .clone()
            .expect("Uninitialized GridFS")
            .bucket()
            .open_download_stream(self.id.clone().into())
            .await
            .or_else(|e| Err::<_, Error>(e.into()))?;

        Ok(GridReader {
            file: self.clone(),
            fs: self.fs.clone().unwrap(),
            stream: reader,
        })
    }

    /// Creates a [GridWriter] to write this file into GridFS. This method takes ownership of the [GridFile], which will be returned by [GridWriter::commit()]
    /// 
    /// <div class="warning">Panics: If the GridFS instance has not been attached.</div>
    pub async fn write(self) -> MResult<GridWriter> {
        let bucket = self
            .fs
            .clone()
            .expect("Uninitialized GridFS")
            .bucket();
        let mut stream = bucket
            .open_upload_stream(self.filename.clone())
            .id(self.id.clone().into());

        if let Some(meta) = self.metadata.clone() {
            stream = stream.metadata(meta);
        }

        let writer = stream.await
            .or_else(|e| Err::<_, Error>(e.into()))?;

        Ok(GridWriter {
            file: self.clone(),
            fs: self.fs.clone().unwrap(),
            stream: writer,
        })
    }

    /// Gets the file's metadata (if present) and attempts to convert it to the specified type. Returns [None] if no metadata exists or if deserialization fails.
    pub fn metadata<T: DeserializeOwned>(&self) -> Option<T> {
        if let Some(meta) = self.metadata.clone() {
            if let Ok(parsed) = from_document::<T>(meta) {
                return Some(parsed);
            }
        }

        None
    }

    /// Returns the raw [bson::Document] of the metadata, if present.
    pub fn untyped_metadata(&self) -> Option<Document> {
        self.metadata.clone()
    }
}

/// A wrapper around [mongodb::gridfs::GridFsUploadStream]
#[pin_project::pin_project]
pub struct GridWriter {
    pub(crate) file: GridFile,
    pub(crate) fs: GridFS,

    #[pin]
    pub(crate) stream: GridFsUploadStream,
}

/// A wrapper around [mongodb::gridfs::GridFsDownloadStream]
#[pin_project::pin_project]
pub struct GridReader {
    pub(crate) file: GridFile,
    pub(crate) fs: GridFS,

    #[pin]
    pub(crate) stream: GridFsDownloadStream,
}

impl AsyncRead for GridReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        this.stream.poll_read(cx, buf)
    }
}

impl AsyncWrite for GridWriter {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.project();
        this.stream.poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        this.stream.poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        this.stream.poll_close(cx)
    }
}

impl GridWriter {
    /// Closes the writer, saves the file to the database, and retrieves the resulting [GridFile]
    pub async fn commit(mut self) -> MResult<GridFile> {
        self.close()
            .await
            .or_else(|e| Err(Error::WriteFailure(e.to_string())))?;
        let info = self
            .fs
            .bucket()
            .find_one(doc! {"_id": self.file.id})
            .await
            .or_else(|e| Err(<mongodb::error::Error as Into<Error>>::into(e)))?
            .ok_or(Error::NotFound)?;
        let mut created = self.file.clone();
        created.details = Some(FileDetails {
            length: info.length,
            chunk_size_bytes: info.chunk_size_bytes,
            upload_date: info.upload_date.to_chrono(),
        });
        Ok(created)
    }
}
