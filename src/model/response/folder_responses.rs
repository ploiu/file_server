use crate::model::db::Folder;
use crate::model::response::file_responses::{FileMetadataResponse, GetFileResponse};
use crate::model::response::BasicMessage;
use rocket::serde::{json::Json, Serialize};
use std::iter;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FolderResponse {
    pub id: u32,
    pub path: String,
    pub folders: Vec<FolderResponse>,
    pub files: Vec<FileMetadataResponse>,
}

impl FolderResponse {
    pub fn from(base: Folder) -> FolderResponse {
        FolderResponse {
            // should always have an id when coming from the database
            id: base.id.unwrap(),
            path: base.name,
            // TODO all nested folders
            folders: Vec::new(),
            // TODO all nested files
            files: Vec::new(),
        }
    }

    pub fn folders(&mut self, folders: Vec<Folder>) {
        folders
            .iter()
            .map(|f| FolderResponse {
                id: f.id.unwrap(),
                folders: Vec::new(),
                path: String::from(&f.name),
                files: Vec::new(),
            })
            .for_each(|f| {
                self.folders.push(f);
            });
    }
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
