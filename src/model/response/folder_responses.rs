use std::collections::HashMap;
use std::fs::File;
use std::ops::AddAssign;

use rocket::serde::{Deserialize, Serialize, json::Json};

use crate::model::api::FileApi;
use crate::model::repository::Folder;
use crate::model::response::{BasicMessage, TagApi};

type NoContent = ();

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
#[serde(crate = "rocket::serde")]
pub struct FolderResponse {
    pub id: u32,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
    pub path: String,
    pub name: String,
    pub folders: Vec<FolderResponse>,
    pub files: Vec<FileApi>,
    pub tags: Vec<TagApi>,
}

impl AddAssign<Vec<TagApi>> for FolderResponse {
    fn add_assign(&mut self, rhs: Vec<TagApi>) {
        self.tags = rhs;
    }
}

impl AddAssign<Vec<FileApi>> for FolderResponse {
    fn add_assign(&mut self, rhs: Vec<FileApi>) {
        self.files = rhs;
    }
}

impl AddAssign<Vec<FolderResponse>> for FolderResponse {
    fn add_assign(&mut self, rhs: Vec<FolderResponse>) {
        self.folders = rhs;
    }
}

impl From<Folder> for FolderResponse {
    fn from(base: Folder) -> Self {
        let split_name = String::from(&base.name);
        let split_name = split_name.split('/');
        let name = String::from(split_name.last().unwrap_or(base.name.as_str()));
        FolderResponse {
            // should always have an id when coming from the database
            id: base.id.unwrap(),
            parent_id: base.parent_id,
            path: String::from(&base.name),
            name,
            folders: Vec::new(),
            files: Vec::new(),
            tags: Vec::new(),
        }
    }
}

#[derive(Responder)]
pub enum GetFolderResponse {
    #[response(status = 404, content_type = "json")]
    FolderNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FolderDbError(Json<BasicMessage>),
    #[response(status = 200)]
    Success(Json<FolderResponse>),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum DownloadFolderResponse {
    #[response(status = 200, content_type = "application/x-tar")]
    Success(File),
    #[response(status = 400, content_type = "json")]
    BadRequest(Json<BasicMessage>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 404, content_type = "json")]
    FolderNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FileSystemError(Json<BasicMessage>),
}

#[derive(Responder)]
pub enum CreateFolderResponse {
    #[response(status = 400, content_type = "json")]
    FolderAlreadyExists(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FolderDbError(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FileSystemError(Json<BasicMessage>),
    #[response(status = 201)]
    Success(Json<FolderResponse>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 404, content_type = "json")]
    ParentNotFound(Json<BasicMessage>),
}

#[derive(Responder)]
pub enum UpdateFolderResponse {
    #[response(status = 400, content_type = "json")]
    FolderAlreadyExists(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FolderDbError(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FileSystemError(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    TagError(Json<BasicMessage>),
    #[response(status = 200)]
    Success(Json<FolderResponse>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 404, content_type = "json")]
    ParentNotFound(Json<BasicMessage>),
    #[response(status = 404, content_type = "json")]
    FolderNotFound(Json<BasicMessage>),
}

#[derive(Responder)]
pub enum DeleteFolderResponse {
    #[response(status = 404, content_type = "json")]
    FolderNotFound(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FolderDbError(Json<BasicMessage>),
    #[response(status = 500, content_type = "json")]
    FileSystemError(Json<BasicMessage>),
    #[response(status = 204)]
    Success(NoContent),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum GetMultiPreviewResponse {
    /// takes a json string of Vec<Vec<u8>>
    #[response(status = 200, content_type = "application/json")]
    Success(Json<HashMap<u32, Vec<u8>>>),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    GenericError(Json<BasicMessage>),
}
