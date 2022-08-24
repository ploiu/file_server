use crate::facade::folder_facade;
use crate::facade::folder_facade::{get_child_folders_for, get_folder_by_id};
use crate::model::db;
use crate::model::request::folder_requests::CreateFolderRequest;
use crate::model::response::folder_responses::FolderResponse;
use crate::service::file_service::FILE_DIR;
use std::fs;
use std::path::Path;

#[derive(PartialEq, Debug)]
pub enum GetFolderError {
    NotFound,
    DbFailure,
}

#[derive(PartialEq, Debug)]
pub enum CreateFolderError {
    /// a folder with the name in the selected path already exists
    AlreadyExists,
    /// the database failed to save the folder
    DbFailure,
    /// the file system failed to write the folder
    FileSystemFailure,
    /// the requested parent folder does not exist
    ParentNotFound,
}

pub fn get_folder(id: u32) -> Result<FolderResponse, GetFolderError> {
    match get_folder_by_id(id) {
        Ok(folder) => {
            let mut folder = FolderResponse {
                // should always have an id when coming from the database
                id: folder.id.unwrap(),
                parent_id: folder.parent_id,
                path: folder.name,
                // TODO all nested folders
                folders: Vec::new(),
                // TODO all nested files
                files: Vec::new(),
            };
            folder.folders(get_child_folders_for(id).unwrap());
            Ok(folder)
        }
        Err(e) => Err(e),
    }
}

pub fn create_folder(folder: &CreateFolderRequest) -> Result<FolderResponse, CreateFolderError> {
    let db_folder = db::Folder {
        id: None,
        name: String::from(&folder.name),
        parent_id: folder.parent_id,
    };
    match folder_facade::create_folder(&db_folder) {
        Ok(f) => {
            let folder_path = format!("{}/{}", FILE_DIR, f.name);
            let fs_path = Path::new(folder_path.as_str());
            match fs::create_dir(fs_path) {
                Ok(_) => Ok(FolderResponse {
                    id: f.id.unwrap(),
                    parent_id: f.parent_id,
                    path: f.name,
                    folders: Vec::new(),
                    files: Vec::new(),
                }),
                Err(_) => Err(CreateFolderError::FileSystemFailure),
            }
        }
        Err(e) => Err(e),
    }
}
