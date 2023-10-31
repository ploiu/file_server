use crate::model::response::Tag;
use rocket::serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateFolderRequest {
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateFolderRequest {
    pub id: u32,
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
    pub tags: Vec<Tag>,
}
