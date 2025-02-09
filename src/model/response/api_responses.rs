use rocket::serde::json::Json;

use crate::{model::response::BasicMessage, service::api_service::DiskInfo};

type NoContent = ();

#[derive(Responder)]
pub enum SetPassWordResponse {
    #[response(status = 201)]
    Created(NoContent),
    #[response(status = 400, content_type = "json")]
    AlreadyExists(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    Failure(Json<BasicMessage>),
}

#[derive(Responder)]
pub enum UpdatePasswordResponse {
    #[response(status = 204)]
    Success(NoContent),
    #[response(status = 401, content_type = "json")]
    Unauthorized(NoContent),
}

#[derive(Responder)]
pub enum GetDiskInfoResponse {
    #[response(status = 200, content_type = "json")]
    Success(Json<DiskInfo>),
    /// windows isn't supported for this endpoint
    #[response(status = 400, content_type = "json")]
    Windows(Json<BasicMessage>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    GenericError(Json<BasicMessage>),
}
