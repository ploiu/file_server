use crate::model::repository::FileRecord;
use crate::model::response::BasicMessage;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use std::fs::File;

type NoContent = ();

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct FileMetadataResponse {
    pub id: u32,
    pub name: String,
}

impl FileMetadataResponse {
    pub fn from(f: &FileRecord) -> FileMetadataResponse {
        FileMetadataResponse {
            id: f.id.unwrap(),
            name: String::from(&f.name),
        }
    }
}

#[derive(Responder)]
pub enum GetFileResponse {
    #[response(status = 404, content_type = "json")]
    FileNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FileDbError(Json<BasicMessage>),
    #[response(status = 200, content_type = "json")]
    Success(Json<FileMetadataResponse>),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum DownloadFileResponse {
    #[response(status = 404, content_type = "json")]
    FileNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FileDbError(Json<BasicMessage>),
    #[response(status = 200)]
    Success(File),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum CreateFileResponse {
    #[response(status = 201)]
    Success(Json<FileMetadataResponse>),
    #[response(status = 400, content_type = "json")]
    BadRequest(Json<BasicMessage>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    Failure(Json<BasicMessage>),
    #[response(status = 404, content_type = "json")]
    NotFound(Json<BasicMessage>),
    #[response(status = 400, content_type = "json")]
    AlreadyExists(Json<BasicMessage>),
}

#[derive(Responder)]
pub enum DeleteFileResponse {
    #[response(status = 204)]
    Deleted(NoContent),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    Failure(Json<BasicMessage>),
    #[response(status = 404, content_type = "json")]
    NotFound(Json<BasicMessage>),
}

#[derive(Responder)]
pub enum UpdateFileResponse {
    #[response(status = 200)]
    Success(Json<FileMetadataResponse>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 400, content_type = "json")]
    BadRequest(Json<BasicMessage>),
    #[response(status = 404, content_type = "json")]
    NotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    GenericError(Json<BasicMessage>),
}

#[derive(Responder)]
pub enum SearchFileResponse {
    #[response(status = 200)]
    Success(Json<Vec<FileMetadataResponse>>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 400, content_type = "json")]
    BadRequest(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    GenericError(Json<BasicMessage>),
}
