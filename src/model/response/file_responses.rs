use std::fs::File;

use crate::model::api::FileApi;
use rocket::serde::json::Json;

use crate::model::response::BasicMessage;

type NoContent = ();

#[derive(Responder)]
pub enum GetFileResponse {
    #[response(status = 404, content_type = "json")]
    FileNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FileDbError(Json<BasicMessage>),
    #[response(status = 200, content_type = "json")]
    Success(Json<FileApi>),
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
    Success(Json<FileApi>),
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
    Success(Json<FileApi>),
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
    Success(Json<Vec<FileApi>>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 400, content_type = "json")]
    BadRequest(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    GenericError(Json<BasicMessage>),
}
