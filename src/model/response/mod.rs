use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::Json;

pub mod api_responses;
pub mod file_responses;
pub mod folder_responses;

/// represents a basic json message
#[derive(Responder, Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct BasicMessage {
    pub(crate) message: String,
}

impl BasicMessage {
    pub fn new(message: &str) -> Json<BasicMessage> {
        Json::from(BasicMessage {
            message: message.to_string(),
        })
    }
}
