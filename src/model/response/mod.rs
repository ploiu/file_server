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

/// represents a tag _on_ a file or folder, not just a standalone tag.
///
/// In order to maintain compatibility with existing clients, the [`id`] field matches the id of the [`Tag`], not the [`TaggedItem`].
/// Since this will be on a file or a folder, that should be enough information to determine which record to modify or remove if needed
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(crate = "rocket::serde")]
pub struct TaggedItemApi {
    /// the id of the tag itself, not the TaggedItemApi. Will be `None` if it's a new tag for that item coming from a client
    pub id: Option<u32>,
    /// the title of the tag
    pub title: String,
    /// the folder this tag is implicated by. Will be None if the tag is explicit
    #[serde(rename = "implicitFrom")]
    pub implicit_from: Option<u32>,
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

impl From<repository::TaggedItem> for TaggedItemApi {
    fn from(value: repository::TaggedItem) -> Self {
        Self {
            id: Some(value.tag_id),
            title: value.title,
            implicit_from: value.implicit_from_id,
        }
    }
}
