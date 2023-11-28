use rocket::serde::json::Json;

use crate::model::response::{BasicMessage, TagApi};

pub type NoContent = ();

#[derive(Responder)]
pub enum GetTagResponse {
    #[response(status = 404, content_type = "json")]
    TagNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    TagDbError(Json<BasicMessage>),
    #[response(status = 200)]
    Success(Json<TagApi>),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum CreateTagResponse {
    #[response(status = 500, content_type = "json")]
    TagDbError(Json<BasicMessage>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 201, content_type = "json")]
    Success(Json<TagApi>),
}

#[derive(Responder)]
pub enum UpdateTagResponse {
    #[response(status = 404, content_type = "json")]
    TagNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    TagDbError(Json<BasicMessage>),
    #[response(status = 400, content_type = "json")]
    TagAlreadyExists(Json<BasicMessage>),
    #[response(status = 200)]
    Success(Json<TagApi>),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum DeleteTagResponse {
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    TagDbError(Json<BasicMessage>),
    #[response(status = 204)]
    Success(NoContent),
}
