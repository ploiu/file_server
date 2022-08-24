use crate::model::response::file_responses::{FileMetadataResponse, GetFileResponse};
use crate::model::response::BasicMessage;
use rocket::serde::{json::Json, Serialize};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FolderResponse {
    pub id: u32,
    pub path: String,
    pub folders: Vec<FolderResponse>,
    pub files: Vec<FileMetadataResponse>,
}

#[derive(Responder)]
pub enum GetFolderResponse {
    #[response(status = 404, content_type = "json")]
    FileNotFound(BasicMessage),
    #[response(status = 500, content_type = "json")]
    FileDbError(BasicMessage),
    #[response(status = 200)]
    Success(Json<FolderResponse>),
    #[response(status = 401)]
    Unauthorized(String),
}
