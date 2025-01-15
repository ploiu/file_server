use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};

use crate::model::repository;

pub mod api_responses;
pub mod file_responses;
pub mod folder_responses;
pub mod tag_responses;

/// represents a basic json message
#[derive(Responder, Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct BasicMessage {
    pub message: String,
}

/// this will be the same no matter if it's a request or a response. This is a bit
/// different than how Files and Folders are
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(crate = "rocket::serde")]
pub struct TagApi {
    /// will be None if new
    pub id: Option<u32>,
    pub title: String,
}

// ----------------------------------

impl BasicMessage {
    pub fn new(message: &str) -> Json<BasicMessage> {
        Json::from(BasicMessage {
            message: message.to_string(),
        })
    }
}

impl From<&str> for BasicMessage {
    fn from(value: &str) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<String> for BasicMessage {
    fn from(value: String) -> Self {
        Self { message: value }
    }
}

impl From<repository::Tag> for TagApi {
    fn from(value: repository::Tag) -> Self {
        TagApi {
            id: Some(value.id),
            title: value.title,
        }
    }
}
