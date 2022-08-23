use crate::model::response::BasicMessage;
use std::fs::File;

type NoContent = ();

#[derive(Responder)]
pub enum GetFileResponse {
    #[response(status = 404, content_type = "json")]
    FileNotFound(BasicMessage),
    #[response(status = 500, content_type = "json")]
    FileDbError(BasicMessage),
    #[response(status = 200)]
    Success(File),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum CreateFileResponse {
    #[response(status = 201)]
    Created(NoContent),
    #[response(status = 400, content_type = "json")]
    BadRequest(BasicMessage),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    Failure(BasicMessage),
}

#[derive(Responder)]
pub enum DeleteFileResponse {
    #[response(status = 204)]
    Deleted(NoContent),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    Failure(BasicMessage),
    #[response(status = 404, content_type = "json")]
    NotFound(BasicMessage),
}
