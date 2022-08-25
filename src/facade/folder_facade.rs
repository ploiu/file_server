use crate::db::folder_repository::{get_by_id, get_child_folders};
use crate::db::{folder_repository, open_connection};
use crate::model::db::Folder;
use crate::service::file_service::FILE_DIR;
use crate::service::folder_service::{CreateFolderError, GetFolderError, UpdateFolderError};

pub fn get_folder_by_id(id: u32) -> Result<Folder, GetFolderError> {
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

pub fn get_child_folders_for(id: u32) -> Result<Vec<Folder>, GetFolderError> {
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
    let mut folder_path: String = String::from(&folder.name);
    // if the folder has a parent id, we need to check if it exists and doesn't have this folder in it
    if let Some(parent_id) = folder.parent_id {
        match get_by_id(parent_id, &con) {
            Ok(parent) => {
                let new_folder_path = format!("{}/{}", parent.name, folder.name);
                folder_path = String::from(&new_folder_path);
                // parent folder exists, now we need to check if there are any child folders with our folder name
                let children = get_child_folders(parent.id.unwrap(), &con).unwrap();
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
    };
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
    let con = open_connection();
    let mut new_path: String = String::from(&folder.name);
    // make sure the folder already exists in the db
    match get_by_id(folder.id.unwrap(), &con) {
        Ok(_) => { /* no op - just confirm it exists */ }
        Err(_) => {
            con.close().unwrap();
            return Err(UpdateFolderError::NotFound);
        }
    }
    // first we need to check if the parent folder exists
    match folder.parent_id {
        Some(parent_id) => match get_by_id(parent_id, &con) {
            Ok(parent) => {
                new_path = format!("{}/{}/{}", FILE_DIR, parent.name, new_path);
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
