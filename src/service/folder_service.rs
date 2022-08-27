use crate::facade::folder_facade;
use crate::model::db;
use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
use crate::model::response::folder_responses::FolderResponse;
use crate::service::file_service::{check_root_dir, FILE_DIR};
use regex::Regex;
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

#[derive(PartialEq, Debug)]
pub enum UpdateFolderError {
    /// a folder with the name in the selected path already exists
    AlreadyExists,
    /// the database failed to update the folder
    DbFailure,
    /// the file system failed to move the folder
    FileSystemFailure,
    /// the requested parent folder does not exist
    ParentNotFound,
    /// The folder could not be found
    NotFound,
    /// The user attempted to do an illegal action, such as moving a parent folder into its own child
    NotAllowed,
}

#[derive(PartialEq, Debug)]
pub enum GetChildFilesError {
    /// database could not execute the query
    DbFailure,
    /// the folder id could not be found
    FolderNotFound,
}

#[derive(PartialEq, Debug)]
pub enum DeleteFolderError {
    /// database could not execute the query
    DbFailure,
    /// folder not in the db
    FolderNotFound,
    /// could not remove the folder from the database
    FileSystemError,
}

#[derive(PartialEq, Debug)]
pub enum LinkFolderError {
    DbError,
}

pub fn get_folder(id: Option<u32>) -> Result<FolderResponse, GetFolderError> {
    match folder_facade::get_folder_by_id(id) {
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
            folder.folders(folder_facade::get_child_folders_for(id).unwrap());
            folder.files(folder_facade::get_files_for_folder(id).unwrap());
            Ok(folder)
        }
        Err(e) => Err(e),
    }
}

pub async fn create_folder(
    folder: &CreateFolderRequest,
) -> Result<FolderResponse, CreateFolderError> {
    check_root_dir(FILE_DIR).await;
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

pub fn update_folder(folder: &UpdateFolderRequest) -> Result<FolderResponse, UpdateFolderError> {
    let original_folder = match folder_facade::get_folder_by_id(Some(folder.id)) {
        Ok(f) => f,
        Err(e) if e == GetFolderError::NotFound => return Err(UpdateFolderError::NotFound),
        _ => return Err(UpdateFolderError::DbFailure),
    };
    let db_folder = db::Folder {
        id: Some(folder.id),
        parent_id: folder.parent_id,
        name: String::from(&folder.name),
    };
    let updated_folder = match folder_facade::update_folder(&db_folder) {
        Ok(updated) => updated,
        Err(e) => return Err(e),
    };
    match fs::rename(
        format!("{}/{}", FILE_DIR, original_folder.name),
        &updated_folder.name,
    ) {
        Ok(_) => { /*no op - move was ok */ }
        Err(e) => {
            eprintln!("Failed to move folder! Nested exception is: \n {:?}", e);
            return Err(UpdateFolderError::FileSystemFailure);
        }
    };

    Ok(FolderResponse {
        id: updated_folder.id.unwrap(),
        folders: Vec::new(),
        files: Vec::new(),
        parent_id: updated_folder.parent_id,
        path: Regex::new(format!("^{}/", FILE_DIR).as_str())
            .unwrap()
            .replace(&updated_folder.name, "")
            .to_string(),
    })
}

pub fn delete_folder(id: u32) -> Result<(), DeleteFolderError> {
    match folder_facade::delete_folder(id) {
        Ok(f) => {
            // delete went well, now time to actually remove the folder
            let path = format!("{}/{}", FILE_DIR, f.name);
            match fs::remove_dir_all(path) {
                Err(e) => {
                    eprintln!("Failed to recursively delete folder from disk! Nested exception is: \n {:?}", e);
                    return Err(DeleteFolderError::FileSystemError);
                }
                _ => { /*no op - folder deleted successfully */ }
            };
        }
        Err(e) => return Err(e),
    };
    Ok(())
}
