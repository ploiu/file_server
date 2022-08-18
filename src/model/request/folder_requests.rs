use rocket::serde::Deserialize;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateFolderRequest {
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateFolderRequest {
    pub id: u32,
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
}
