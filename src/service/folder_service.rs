use std::fs;
use std::path::Path;

use regex::Regex;
use rusqlite::Connection;

use model::repository::Folder;

use crate::model::error::file_errors::DeleteFileError;
use crate::model::error::folder_errors::{
    CreateFolderError, DeleteFolderError, GetChildFilesError, GetFolderError, UpdateFolderError,
};
use crate::model::repository::FileRecord;
use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
use crate::model::response::folder_responses::FolderResponse;
use crate::repository::folder_repository;
use crate::service::file_service;
use crate::service::file_service::{check_root_dir, FILE_DIR};
use crate::{model, repository};

pub async fn get_folder(id: Option<u32>) -> Result<FolderResponse, GetFolderError> {
    let db_id = if Some(0) == id || id.is_none() {
        None
    } else {
        id
    };
    check_root_dir(FILE_DIR).await;
    let folder = get_folder_by_id(db_id)?;
    let mut folder = FolderResponse {
        // should always have an id when coming from the database
        id: folder.id.unwrap(),
        parent_id: folder.parent_id,
        path: folder.name,
        folders: Vec::new(),
        files: Vec::new(),
    };
    let con = repository::open_connection();
    let child_folders = folder_repository::get_child_folders(db_id, &con).map_err(|e| {
        eprintln!(
            "Failed to pull child folder info from database! Nested exception is: \n {:?}",
            e
        );
        GetFolderError::DbFailure
    });
    con.close().unwrap();
    folder.folders(child_folders?);
    folder.files(get_files_for_folder(db_id).unwrap());
    Ok(folder)
}

pub async fn create_folder(
    folder: &CreateFolderRequest,
) -> Result<FolderResponse, CreateFolderError> {
    check_root_dir(FILE_DIR).await;
    // the client can pass 0 for the folder id, in which case it needs to be translated to None for the database
    let db_folder = if let Some(0) = folder.parent_id {
        None
    } else {
        folder.parent_id
    };
    let db_folder = Folder {
        id: None,
        name: String::from(&folder.name),
        parent_id: db_folder,
    };
    match create_folder_internal(&db_folder) {
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
    if folder.id == 0 {
        return Err(UpdateFolderError::NotFound);
    }
    let original_folder = match get_folder_by_id(Some(folder.id)) {
        Ok(f) => f,
        Err(e) if e == GetFolderError::NotFound => return Err(UpdateFolderError::NotFound),
        _ => return Err(UpdateFolderError::DbFailure),
    };
    let db_folder = Folder {
        id: Some(folder.id),
        parent_id: folder.parent_id,
        name: String::from(&folder.name),
    };
    if db_folder.parent_id == db_folder.id {
        return Err(UpdateFolderError::NotAllowed);
    }
    let updated_folder = update_folder_internal(&db_folder)?;
    // if we can't rename the folder, then we have problems
    if let Err(e) = fs::rename(
        format!("{}/{}", FILE_DIR, original_folder.name),
        &updated_folder.name,
    ) {
        eprintln!("Failed to move folder! Nested exception is: \n {:?}", e);
        return Err(UpdateFolderError::FileSystemFailure);
    }
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
    if id == 0 {
        return Err(DeleteFolderError::FolderNotFound);
    }
    let con = repository::open_connection();
    let deleted_folder = delete_folder_recursively(id, &con);
    con.close().unwrap();
    let deleted_folder = deleted_folder?;
    // delete went well, now time to actually remove the folder
    let path = format!("{}/{}", FILE_DIR, deleted_folder.name);
    if let Err(e) = fs::remove_dir_all(path) {
        eprintln!(
            "Failed to recursively delete folder from disk! Nested exception is: \n {:?}",
            e
        );
        return Err(DeleteFolderError::FileSystemError);
    };
    Ok(())
}

// private functions
fn get_folder_by_id(id: Option<u32>) -> Result<Folder, GetFolderError> {
    // the client can pass 0 for the folder id, in which case it needs to be translated to None for the database
    let db_folder = if let Some(0) = id { None } else { id };
    let con = repository::open_connection();
    let result = match folder_repository::get_by_id(db_folder, &con) {
        Ok(folder) => Ok(folder),
        Err(error) if error == rusqlite::Error::QueryReturnedNoRows => {
            Err(GetFolderError::NotFound)
        }
        Err(err) => {
            eprintln!(
                "Failed to pull folder info from database! Nested exception is: \n {:?}",
                err
            );
            Err(GetFolderError::DbFailure)
        }
    };
    con.close().unwrap();
    result
}

fn create_folder_internal(folder: &Folder) -> Result<Folder, CreateFolderError> {
    let con = repository::open_connection();
    // make sure the folder doesn't exist
    let mut folder_path: String = String::from(&folder.name);
    // if the folder has a parent id, we need to check if it exists and doesn't have this folder in it
    if let Some(parent_id) = folder.parent_id {
        match folder_repository::get_by_id(Some(parent_id), &con) {
            Ok(parent) => {
                let new_folder_path = format!("{}/{}", parent.name, folder.name);
                folder_path = String::from(&new_folder_path);
                // parent folder exists, now we need to check if there are any child folders with our folder name
                let children = folder_repository::get_child_folders(parent.id, &con).unwrap();
                for child in children.iter() {
                    if &new_folder_path == &child.name {
                        con.close().unwrap();
                        return Err(CreateFolderError::AlreadyExists);
                    }
                }
            }
            _ => {
                con.close().unwrap();
                return Err(CreateFolderError::ParentNotFound);
            }
        };
    } else if Path::new(format!("{}/{}", FILE_DIR, folder_path).as_str()).exists() {
        con.close().unwrap();
        return Err(CreateFolderError::AlreadyExists);
    }
    let created = match folder_repository::create_folder(folder, &con) {
        Ok(f) => {
            // so that I don't have to make yet another repository query to get parent folder path
            Ok(Folder {
                id: f.id,
                parent_id: f.parent_id,
                name: folder_path,
            })
        }
        Err(e) => {
            eprintln!("Error trying to save folder!\nException is: {:?}", e);
            Err(CreateFolderError::DbFailure)
        }
    };
    con.close().unwrap();
    created
}

fn update_folder_internal(folder: &Folder) -> Result<Folder, UpdateFolderError> {
    let con = repository::open_connection();
    let mut new_path: String = String::from(&folder.name);
    // make sure the folder already exists in the repository
    if let Err(_) = folder_repository::get_by_id(folder.id, &con) {
        con.close().unwrap();
        return Err(UpdateFolderError::NotFound);
    }
    let parent_id = if Some(0) == folder.parent_id || folder.parent_id.is_none() {
        None
    } else {
        folder.parent_id
    };

    // first we need to check if the parent folder exists
    match parent_id {
        Some(parent_id) => match folder_repository::get_by_id(Some(parent_id), &con) {
            // parent folder exists, make sure it's not a child folder
            Ok(parent) => {
                // make sure a folder with our name doesn't exist
                let already_exists = match does_folder_exist(&folder.name, parent.id, &con) {
                    Ok(exists) => exists,
                    Err(_e) => {
                        con.close().unwrap();
                        return Err(UpdateFolderError::DbFailure);
                    }
                };
                if already_exists {
                    return Err(UpdateFolderError::AlreadyExists);
                }
                // check to make sure we're not moving to a sub-child
                let check =
                    is_attempt_move_to_sub_child(&folder.id.unwrap(), &parent.id.unwrap(), &con);
                if check == Ok(true) {
                    new_path = format!("{}/{}/{}", FILE_DIR, parent.name, new_path);
                } else if check == Ok(false) {
                    con.close().unwrap();
                    return Err(UpdateFolderError::NotAllowed);
                } else if let Err(e) = check {
                    con.close().unwrap();
                    return Err(e);
                }
            }
            Err(_) => {
                con.close().unwrap();
                return Err(UpdateFolderError::ParentNotFound);
            }
        },
        None => {
            // make sure a folder with our name doesn't exist
            let already_exists = match does_folder_exist(&folder.name, None, &con) {
                Ok(exists) => exists,
                Err(_e) => {
                    con.close().unwrap();
                    return Err(UpdateFolderError::DbFailure);
                }
            };
            if already_exists {
                return Err(UpdateFolderError::AlreadyExists);
            }
            new_path = format!("{}/{}", FILE_DIR, new_path);
        }
    };
    let update = folder_repository::update_folder(
        &Folder {
            id: folder.id,
            name: String::from(&folder.name),
            parent_id,
        },
        &con,
    );
    if update.is_err() {
        con.close().unwrap();
        eprintln!(
            "Failed to update folder in database. Nested exception is: \n {:?}",
            update.unwrap_err()
        );
        return Err(UpdateFolderError::DbFailure);
    }
    con.close().unwrap();
    Ok(Folder {
        id: folder.id,
        parent_id: folder.parent_id,
        name: new_path,
    })
}

/// checks if a folder with the passed name exists within the folder with the passed id
fn does_folder_exist(
    name: &String,
    id: Option<u32>,
    con: &Connection,
) -> Result<bool, rusqlite::Error> {
    let matching_folder = folder_repository::get_child_folders(id, con)?
        .iter()
        .map(|folder| Folder {
            id: folder.id,
            parent_id: folder.parent_id,
            name: String::from(folder.name.to_lowercase().split('/').last().unwrap()),
        })
        .find(|folder| folder.name == name.to_lowercase().split('/').last().unwrap());
    Ok(matching_folder.is_some())
}

/// checks if the new_parent_id being passed matches any id of any sub child of the passed folder_id
fn is_attempt_move_to_sub_child(
    folder_id: &u32,
    new_parent_id: &u32,
    con: &Connection,
) -> Result<bool, UpdateFolderError> {
    return match folder_repository::get_all_child_folder_ids(*folder_id, con) {
        Ok(ids) => {
            if ids.contains(new_parent_id) {
                Err(UpdateFolderError::NotAllowed)
            } else {
                Ok(true)
            }
        }
        _ => Err(UpdateFolderError::DbFailure),
    };
}

/// returns the top-level files for the passed folder
fn get_files_for_folder(id: Option<u32>) -> Result<Vec<FileRecord>, GetChildFilesError> {
    let con = repository::open_connection();
    // first we need to check the folder exists
    if let Err(e) = folder_repository::get_by_id(id, &con) {
        con.close().unwrap();
        return if e == rusqlite::Error::QueryReturnedNoRows {
            Err(GetChildFilesError::FolderNotFound)
        } else {
            eprintln!(
                "Failed to query database for folders. Nested exception is: \n {:?}",
                e
            );
            Err(GetChildFilesError::DbFailure)
        };
    }
    // now we can retrieve all the file records in this folder
    let result = match folder_repository::get_files_for_folder(id, &con) {
        Ok(files) => files,
        Err(e) => {
            con.close().unwrap();
            eprintln!(
                "Failed to query database for child files. Nested exception is: \n {:?}",
                e
            );
            return Err(GetChildFilesError::DbFailure);
        }
    };
    con.close().unwrap();
    Ok(result)
}

/// the main body of `delete_folder`. Takes a connection so that we're not creating a connection on every stack frame
fn delete_folder_recursively(id: u32, con: &Connection) -> Result<Folder, DeleteFolderError> {
    let folder = folder_repository::get_by_id(Some(id), &con).or_else(|e| {
        eprintln!(
            "Failed to recursively delete folder. Nested exception is {:?}",
            e
        );
        return if e == rusqlite::Error::QueryReturnedNoRows {
            Err(DeleteFolderError::FolderNotFound)
        } else {
            Err(DeleteFolderError::DbFailure)
        };
    })?;
    // now that we have the folder, we can delete all the files for that folder
    let files = folder_repository::get_files_for_folder(Some(id), con)
        .or_else(|_| Err(DeleteFolderError::DbFailure))?;
    for file in files.iter() {
        match file_service::delete_file_by_id_with_connection(file.id.unwrap(), con) {
            Err(e) if e == DeleteFileError::NotFound => {}
            Err(_) => return Err(DeleteFolderError::DbFailure),
            Ok(_) => { /*no op - file was removed properly*/ }
        };
    }
    // now that we've deleted all files, we can try with all folders
    let sub_folders = folder_repository::get_child_folders(Some(id), con)
        .or_else(|_| Err(DeleteFolderError::DbFailure))?;
    for sub_folder in sub_folders.iter() {
        delete_folder_recursively(sub_folder.id.unwrap(), con)?;
    }
    // now that we've deleted everything beneath it, delete the requested folder from the repository
    if let Err(e) = folder_repository::delete_folder(id, con) {
        eprintln!(
            "Failed to delete root folder in recursive folder delete. Nested exception is: \n {:?}",
            e
        );
        return Err(DeleteFolderError::DbFailure);
    };
    Ok(folder)
}
