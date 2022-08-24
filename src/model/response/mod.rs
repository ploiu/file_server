pub mod api_responses;
pub mod file_responses;
pub mod folder_responses;

/// represents a basic json message
#[derive(Responder)]
#[response(content_type = "json")]
pub struct BasicMessage {
    message: String,
}

impl BasicMessage {
    pub fn new(message: &str) -> BasicMessage {
        BasicMessage {
            message: message.to_string(),
        }
    }
}
