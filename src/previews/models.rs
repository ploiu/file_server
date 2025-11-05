use rocket::{response::stream::Event, serde::json::Json};

use crate::model::response::BasicMessage;
use base64::{Engine as _, engine::general_purpose};

#[derive(Responder)]
pub enum GetPreviewResponse {
    #[response(status = 200, content_type = "image/png")]
    Success(Vec<u8>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 404, content_type = "json")]
    NotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    GenericError(Json<BasicMessage>),
}

/// Contains the data needed to send a server-sent event (SSE) for a file preview.
///
/// Using SSE allows us to have less data in memory at a time and send data to clients at a
/// sustainable rate for the server
pub struct PreviewEvent {
    /// the id of the file the preview is for
    pub id: u32,
    /// the UTF-8 bytes of the preview
    pub data: Vec<u8>,
}

impl From<PreviewEvent> for Event {
    fn from(val: PreviewEvent) -> Self {
        let base64 = general_purpose::STANDARD.encode(val.data);
        Event::data(base64).id(val.id.to_string())
    }
}

#[derive(Responder, Debug)]
pub enum GetFolderPreviewsError {
    #[response(status = 500)]
    Database(Json<BasicMessage>),
    #[response(status = 401)]
    Unauthorized(String),
    /// no folder with the passed id was found
    #[response(status = 404, content_type = "json")]
    NotFound(Json<BasicMessage>),
}
