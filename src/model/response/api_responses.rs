use crate::model::response::BasicMessage;
use rocket::serde::json::Json;

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
