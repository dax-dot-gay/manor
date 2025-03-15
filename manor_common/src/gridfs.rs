use bson::{Document, doc};
use chrono::Utc;
use futures_util::{AsyncRead, AsyncWrite, AsyncWriteExt};
use mongodb::gridfs::{GridFsBucket, GridFsDownloadStream, GridFsUploadStream};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    client::Client,
    error::{Error, MResult},
};

#[derive(Clone, Debug)]
pub struct GridFS {
    pub(crate) bucket: GridFsBucket,
    pub(crate) client: Client,
    pub(crate) name: String,
}

impl GridFS {
    pub fn bucket(&self) -> GridFsBucket {
        self.bucket.clone()
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

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

    pub async fn delete(&self, id: impl AsRef<Uuid>) -> MResult<()> {
        self.bucket()
            .delete(id.as_ref().into())
            .await
            .or_else(|e| Err(e.into()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileDetails {
    pub length: u64,
    pub chunk_size_bytes: u32,
    pub upload_date: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridFile {
    pub id: Uuid,
    pub filename: String,
    pub details: Option<FileDetails>,
    pub metadata: Option<Document>,

    #[serde(skip)]
    pub(crate) fs: Option<GridFS>,
}

impl GridFile {
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

    pub async fn write(self) -> MResult<GridWriter> {
        let writer = self
            .fs
            .clone()
            .expect("Uninitialized GridFS")
            .bucket()
            .open_upload_stream(self.filename.clone())
            .id(self.id.clone().into())
            .await
            .or_else(|e| Err::<_, Error>(e.into()))?;
        Ok(GridWriter {
            file: self.clone(),
            fs: self.fs.clone().unwrap(),
            stream: writer,
        })
    }
}

#[pin_project::pin_project]
pub struct GridWriter {
    pub(crate) file: GridFile,
    pub(crate) fs: GridFS,

    #[pin]
    pub(crate) stream: GridFsUploadStream,
}

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
