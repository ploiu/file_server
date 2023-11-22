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
    pub(crate) message: String,
}

/// this will be the same no matter if it's a request or a response. This is a bit
/// different than how Files and Folders are
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct TagApi {
    /// will be None if new
    pub id: Option<u32>,
    pub title: String,
}

impl Clone for TagApi {
    fn clone(&self) -> Self {
        TagApi {
            id: self.id.clone(),
            title: self.title.clone(),
        }
    }
}

// ----------------------------------

impl BasicMessage {
    pub fn new(message: &str) -> Json<BasicMessage> {
        Json::from(BasicMessage {
            message: message.to_string(),
        })
    }
}

impl TagApi {
    pub fn from(orig: repository::Tag) -> TagApi {
        TagApi {
            id: Some(orig.id),
            title: orig.title,
        }
    }
}
