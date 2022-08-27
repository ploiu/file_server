use crate::db::folder_repository::{get_by_id, get_child_folders};
use crate::db::{folder_repository, open_connection};
use crate::facade::file_facade;
use crate::model::db;
use crate::model::db::Folder;
use crate::service::file_service::{DeleteFileError, FILE_DIR};
use crate::service::folder_service::{
    CreateFolderError, DeleteFolderError, GetChildFilesError, GetFolderError, LinkFolderError,
    UpdateFolderError,
};
use rusqlite::Connection;
use std::path::Path;

pub fn get_folder_by_id(id: Option<u32>) -> Result<Folder, GetFolderError> {
    let con = open_connection();
    let result = match get_by_id(id, &con) {
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

pub fn get_child_folders_for(id: Option<u32>) -> Result<Vec<Folder>, GetFolderError> {
    let con = open_connection();
    let result = match get_child_folders(id, &con) {
        Ok(folder) => Ok(folder),
        Err(err) => {
            eprintln!(
                "Failed to pull child folder info from database! Nested exception is: \n {:?}",
                err
            );
            Err(GetFolderError::DbFailure)
        }
    };
    con.close().unwrap();
    result
}

pub fn create_folder(folder: &Folder) -> Result<Folder, CreateFolderError> {
    let con = open_connection();
    // make sure the folder doesn't exist
    let mut folder_path: String = String::from(&folder.name);
    // if the folder has a parent id, we need to check if it exists and doesn't have this folder in it
    if let Some(parent_id) = folder.parent_id {
        match get_by_id(Some(parent_id), &con) {
            Ok(parent) => {
                let new_folder_path = format!("{}/{}", parent.name, folder.name);
                folder_path = String::from(&new_folder_path);
                // parent folder exists, now we need to check if there are any child folders with our folder name
                let children = get_child_folders(parent.id, &con).unwrap();
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
        return Err(CreateFolderError::AlreadyExists);
    }
    let created = match folder_repository::create_folder(&folder, &con) {
        Ok(f) => {
            // so that I don't have to make yet another db query to get parent folder path
            Ok(Folder {
                id: f.id,
                parent_id: f.parent_id,
                name: folder_path,
            })
        }
        _ => Err(CreateFolderError::DbFailure),
    };
    con.close().unwrap();
    created
}

/// updates the folder record in the database, returning a new folder with the updated path
pub fn update_folder(folder: &Folder) -> Result<Folder, UpdateFolderError> {
    if folder.parent_id == folder.id {
        return Err(UpdateFolderError::NotAllowed);
    }
    let con = open_connection();
    let mut new_path: String = String::from(&folder.name);
    // make sure the folder already exists in the db
    match get_by_id(folder.id, &con) {
        Ok(_) => { /* no op - just confirm it exists */ }
        Err(_) => {
            con.close().unwrap();
            return Err(UpdateFolderError::NotFound);
        }
    }
    // first we need to check if the parent folder exists
    match folder.parent_id {
        Some(parent_id) => match get_by_id(Some(parent_id), &con) {
            // parent folder exists, make sure it's not a child folder
            Ok(parent) => {
                match folder_repository::get_all_child_folder_ids(folder.id.unwrap(), &con) {
                    Ok(ids) => {
                        if ids.contains(&folder.parent_id.unwrap()) {
                            con.close().unwrap();
                            return Err(UpdateFolderError::NotAllowed);
                        } else {
                            new_path = format!("{}/{}/{}", FILE_DIR, parent.name, new_path);
                        }
                    }
                    _ => {
                        con.close().unwrap();
                        return Err(UpdateFolderError::DbFailure);
                    }
                };
            }
            Err(_) => {
                con.close().unwrap();
                return Err(UpdateFolderError::ParentNotFound);
            }
        },
        None => {
            new_path = format!("{}/{}", FILE_DIR, new_path);
        }
    };
    let update = folder_repository::update_folder(&folder, &con);
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

/// retrieves all the top-level files for the passed folder and returns them
pub fn get_files_for_folder(id: Option<u32>) -> Result<Vec<db::FileRecord>, GetChildFilesError> {
    let con = open_connection();
    // first we need to check the folder exists
    match folder_repository::get_by_id(id, &con) {
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => {
            con.close().unwrap();
            return Err(GetChildFilesError::FolderNotFound);
        }
        Err(e) => {
            con.close().unwrap();
            eprintln!(
                "Failed to query database for folders. Nested exception is: \n {:?}",
                e
            );
            return Err(GetChildFilesError::DbFailure);
        }
        _ => { /* no op - we confirm the folder exists */ }
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
    Ok(result)
}

/// recursively deletes all files and folders under the folder with the passed id
pub fn delete_folder(id: u32) -> Result<db::Folder, DeleteFolderError> {
    let con = open_connection();
    let result = delete_folder_recursively(id, &con);
    con.close().unwrap();
    result
}

/// the main body of `delete_folder`. Takes a connection so that we're not creating a connection on every stack frame
fn delete_folder_recursively(id: u32, con: &Connection) -> Result<db::Folder, DeleteFolderError> {
    let folder = match get_by_id(Some(id), &con) {
        Ok(f) => f,
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => {
            return Err(DeleteFolderError::FolderNotFound)
        }
        Err(e) => {
            eprintln!(
                "failed to recursively delete folder! Nested exception is: \n {:?}",
                e
            );
            return Err(DeleteFolderError::DbFailure);
        }
    };
    // now that we have the folder, we can delete all the files for that folder
    let files = match folder_repository::get_files_for_folder(Some(id), &con) {
        Ok(f) => f,
        Err(_) => return Err(DeleteFolderError::DbFailure),
    };
    for file in files.iter() {
        match file_facade::delete_file_by_id_with_connection(file.id.unwrap(), &con) {
            Err(e) if e == DeleteFileError::NotFound => {}
            Err(_) => return Err(DeleteFolderError::DbFailure),
            Ok(_) => { /*no op - file was removed properly*/ }
        };
    }
    // now that we've deleted all files, we can try with all folders
    let sub_folders = match folder_repository::get_child_folders(Some(id), &con) {
        Ok(f) => f,
        Err(_) => return Err(DeleteFolderError::DbFailure),
    };
    for sub_folder in sub_folders.iter() {
        delete_folder_recursively(sub_folder.id.unwrap(), &con)?;
    }
    // now that we've deleted everything beneath it, delete the requested folder from the db
    match folder_repository::delete_folder(id, &con) {
        Ok(()) => { /*no op - folder deleted*/ }
        Err(e) => {
            eprintln!("Failed to delete root folder in recursive folder delete. Nested exception is: \n {:?}", e);
            return Err(DeleteFolderError::DbFailure);
        }
    };
    Ok(folder)
}

pub fn link_folder_to_file(file_id: u32, folder_id: u32) -> Result<(), LinkFolderError> {
    let con = open_connection();
    let result = match folder_repository::link_folder_to_file(file_id, folder_id, &con) {
        Ok(()) => Ok(()),
        Err(_) => Err(LinkFolderError::DbError),
    };
    con.close().unwrap();
    return result;
}
