use rocket::serde::{Deserialize, Serialize};

use crate::model::response::TaggedItemApi;

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateFolderRequest {
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
}

/// Intentional narrowing of [`crate::model::response::folder_responses::FolderResponse`]
///
/// This narrowing allows us to safely handle requests to update a folder without worry of accidentally changing fields that shouldn't be changed
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateFolderRequest {
    pub id: u32,
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
    pub tags: Vec<TaggedItemApi>,
}
