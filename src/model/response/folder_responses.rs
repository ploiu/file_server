use crate::model::db::Folder;
use crate::model::response::file_responses::FileMetadataResponse;
use crate::model::response::BasicMessage;
use rocket::serde::{json::Json, Serialize};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FolderResponse {
    pub id: u32,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
    pub path: String,
    pub folders: Vec<FolderResponse>,
    pub files: Vec<FileMetadataResponse>,
}

impl FolderResponse {
    pub fn from(base: &Folder) -> FolderResponse {
        FolderResponse {
            // should always have an id when coming from the database
            id: base.id.unwrap(),
            parent_id: base.parent_id,
            path: String::from(&base.name),
            // TODO all nested folders
            folders: Vec::new(),
            // TODO all nested files
            files: Vec::new(),
        }
    }

    pub fn folders(&mut self, folders: Vec<Folder>) {
        folders
            .iter()
            .map(FolderResponse::from)
            .for_each(|f| self.folders.push(f));
    }
}

#[derive(Responder)]
pub enum GetFolderResponse {
    #[response(status = 404, content_type = "json")]
    FolderNotFound(BasicMessage),
    #[response(status = 500, content_type = "json")]
    FolderDbError(BasicMessage),
    #[response(status = 200)]
    Success(Json<FolderResponse>),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum CreateFolderResponse {
    #[response(status = 400, content_type = "json")]
    FolderAlreadyExists(BasicMessage),
    #[response(status = 500, content_type = "json")]
    FolderDbError(BasicMessage),
    #[response(status = 500, content_type = "json")]
    FileSystemError(BasicMessage),
    #[response(status = 200)]
    Success(Json<FolderResponse>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 404, content_type = "json")]
    ParentNotFound(BasicMessage),
}

#[derive(Responder)]
pub enum UpdateFolderResponse {
    #[response(status = 400, content_type = "json")]
    FolderAlreadyExists(BasicMessage),
    #[response(status = 500, content_type = "json")]
    FolderDbError(BasicMessage),
    #[response(status = 500, content_type = "json")]
    FileSystemError(BasicMessage),
    #[response(status = 200)]
    Success(Json<FolderResponse>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 404, content_type = "json")]
    ParentNotFound(BasicMessage),
    #[response(status = 404, content_type = "json")]
    FolderNotFound(BasicMessage),
}
