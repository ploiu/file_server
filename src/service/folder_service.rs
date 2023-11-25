use std::fs;
use std::path::Path;

use regex::Regex;
use rusqlite::Connection;

use model::repository::Folder;

use crate::model::api::FileApi;
use crate::model::error::file_errors::DeleteFileError;
use crate::model::error::folder_errors::{
    CreateFolderError, DeleteFolderError, GetChildFilesError, GetFolderError, UpdateFolderError,
};
use crate::model::repository::FileRecord;
use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
use crate::model::response::folder_responses::FolderResponse;
use crate::repository::{folder_repository, open_connection};
use crate::service::file_service::{check_root_dir, file_dir};
use crate::service::{file_service, tag_service};
use crate::{model, repository};

pub async fn get_folder(id: Option<u32>) -> Result<FolderResponse, GetFolderError> {
    let db_id = if Some(0) == id || id.is_none() {
        None
    } else {
        id
    };
    check_root_dir(file_dir()).await;
    let folder = get_folder_by_id(db_id)?;
    let mut folder = FolderResponse::from(&folder);
    let con: Connection = repository::open_connection();
    let child_folders = folder_repository::get_child_folders(db_id, &con).map_err(|e| {
        eprintln!(
            "Failed to pull child folder info from database! Nested exception is: \n {:?}",
            e
        );
        GetFolderError::DbFailure
    });
    let tag_db_id = if id.is_none() { 0 } else { id.unwrap() };
    let tags = match tag_service::get_tags_on_folder(tag_db_id) {
        Ok(t) => t,
        Err(_) => {
            con.close().unwrap();
            return Err(GetFolderError::TagError);
        }
    };
    con.close().unwrap();
    folder.folders(child_folders?);
    folder.files(get_files_for_folder(db_id).unwrap());
    for tag in tags {
        folder.tags.push(tag);
    }
    Ok(folder)
}

pub async fn create_folder(
    folder: &CreateFolderRequest,
) -> Result<FolderResponse, CreateFolderError> {
    check_root_dir(file_dir()).await;
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
            let folder_path = format!("{}/{}", file_dir(), f.name);
            let fs_path = Path::new(folder_path.as_str());
            match fs::create_dir(fs_path) {
                Ok(_) => Ok(FolderResponse::from(&f)),
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
        name: folder.name.to_string(),
    };
    if db_folder.parent_id == db_folder.id {
        return Err(UpdateFolderError::NotAllowed);
    }
    let updated_folder = update_folder_internal(&db_folder)?;
    // if we can't rename the folder, then we have problems
    if let Err(e) = fs::rename(
        format!("{}/{}", file_dir(), original_folder.name),
        &updated_folder.name,
    ) {
        eprintln!("Failed to move folder! Nested exception is: \n {:?}", e);
        return Err(UpdateFolderError::FileSystemFailure);
    }
    // updated folder name will be a path, so we need to get just the folder name
    let split_name = String::from(&updated_folder.name);
    let split_name = split_name.split("/");
    let name = String::from(split_name.last().unwrap_or(updated_folder.name.as_str()));
    match tag_service::update_folder_tags(updated_folder.id.unwrap(), folder.tags.clone()) {
        Ok(()) => { /*no op*/ }
        Err(_) => {
            return Err(UpdateFolderError::TagError);
        }
    };
    Ok(FolderResponse {
        id: updated_folder.id.unwrap(),
        folders: Vec::new(),
        files: Vec::new(),
        parent_id: updated_folder.parent_id,
        path: Regex::new(format!("^{}/", file_dir()).as_str())
            .unwrap()
            .replace(&updated_folder.name, "")
            .to_string(),
        name,
        tags: folder.tags.clone(),
    })
}

pub fn folder_exists(id: Option<u32>) -> bool {
    let con: Connection = open_connection();
    let db_id = if Some(0) == id || id.is_none() {
        None
    } else {
        id
    };
    let res = folder_repository::get_by_id(db_id, &con);
    con.close().unwrap();
    return !res.is_err();
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
    let path = format!("{}/{}", file_dir(), deleted_folder.name);
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
    } else if Path::new(format!("{}/{}", file_dir(), folder_path).as_str()).exists() {
        con.close().unwrap();
        return Err(CreateFolderError::AlreadyExists);
    }
    let created = match folder_repository::create_folder(folder, &con) {
        Ok(f) => {
            Ok(Folder {
                id: f.id,
                parent_id: f.parent_id,
                // so that I don't have to make yet another repository query to get parent folder path
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
                let existing_folder = match search_folder_within(&folder.name, parent.id, &con) {
                    Ok(f) => f,
                    Err(_) => {
                        con.close().unwrap();
                        return Err(UpdateFolderError::DbFailure);
                    }
                };
                if existing_folder.is_some() && existing_folder.unwrap().id != folder.id {
                    return Err(UpdateFolderError::AlreadyExists);
                }
                // make sure we're not renaming to a file that already exists in the target parent directory
                let file_already_exists = match does_file_exist(&folder.name, parent.id, &con) {
                    Ok(exists) => exists,
                    Err(_e) => {
                        con.close().unwrap();
                        return Err(UpdateFolderError::DbFailure);
                    }
                };
                if file_already_exists {
                    return Err(UpdateFolderError::FileAlreadyExists);
                }
                // check to make sure we're not moving to a sub-child
                let check =
                    is_attempt_move_to_sub_child(&folder.id.unwrap(), &parent.id.unwrap(), &con);
                if check == Ok(true) {
                    new_path = format!("{}/{}/{}", file_dir(), parent.name, new_path);
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
            let existing_folder = match search_folder_within(&folder.name, None, &con) {
                Ok(f) => f,
                Err(_) => {
                    con.close().unwrap();
                    return Err(UpdateFolderError::DbFailure);
                }
            };
            if existing_folder.is_some() && existing_folder.unwrap().id != folder.id {
                return Err(UpdateFolderError::AlreadyExists);
            }
            // make sure we're not renaming to a file that already exists in the target parent directory
            let file_already_exists = match does_file_exist(&folder.name, None, &con) {
                Ok(exists) => exists,
                Err(_e) => {
                    con.close().unwrap();
                    return Err(UpdateFolderError::DbFailure);
                }
            };
            if file_already_exists {
                return Err(UpdateFolderError::FileAlreadyExists);
            }
            new_path = format!("{}/{}", file_dir(), new_path);
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

fn search_folder_within(
    name: &String,
    parent_id: Option<u32>,
    con: &Connection,
) -> Result<Option<Folder>, rusqlite::Error> {
    let matching_folder = folder_repository::get_child_folders(parent_id, con)?
        .iter()
        .map(|folder| Folder {
            id: folder.id,
            parent_id: folder.parent_id,
            name: String::from(folder.name.to_lowercase().split('/').last().unwrap()),
        })
        .find(|folder| folder.name == name.to_lowercase().split('/').last().unwrap());
    Ok(matching_folder)
}

fn does_file_exist(
    name: &String,
    folder_id: Option<u32>,
    con: &Connection,
) -> Result<bool, rusqlite::Error> {
    let matching_file = folder_repository::get_child_files(folder_id, &con)?
        .iter()
        // this is required because apparently the file is dropped immediately when it's used...
        .map(|file| FileRecord {
            id: file.id,
            name: String::from(&file.name),
        })
        .find(|file| file.name == name.to_lowercase());
    Ok(matching_file.is_some())
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
fn get_files_for_folder(id: Option<u32>) -> Result<Vec<FileApi>, GetChildFilesError> {
    let con: Connection = repository::open_connection();
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
    let child_files = match folder_repository::get_child_files(id, &con) {
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
    let mut result: Vec<FileApi> = Vec::new();
    for file in child_files {
        let tags = match tag_service::get_tags_on_file(file.id.unwrap()) {
            Ok(t) => t,
            Err(_) => {
                con.close().unwrap();
                return Err(GetChildFilesError::TagError);
            }
        };
        result.push(FileApi::from(file, tags))
    }
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
    let files = folder_repository::get_child_files(Some(id), con)
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

#[cfg(test)]
mod get_folder_tests {
    use rocket::tokio;

    use crate::model::error::folder_errors::GetFolderError;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::TagApi;
    use crate::service::folder_service::get_folder;
    use crate::test::{cleanup, create_folder_db_entry, create_tag_folder, refresh_db};

    #[tokio::test]
    async fn get_folder_works() {
        refresh_db();
        create_folder_db_entry("test", None);
        let folder = get_folder(Some(1)).await.unwrap();
        assert_eq!(
            FolderResponse {
                id: 1,
                parent_id: None,
                path: "test".to_string(),
                name: "test".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![],
            },
            folder
        );
        cleanup();
    }

    #[tokio::test]
    async fn get_folder_not_found() {
        refresh_db();
        let err = get_folder(Some(1)).await.unwrap_err();
        assert_eq!(GetFolderError::NotFound, err);
        cleanup();
    }

    #[tokio::test]
    async fn get_folder_retrieves_tags() {
        refresh_db();
        create_folder_db_entry("test", None);
        create_tag_folder("tag1", 1);
        let expected = FolderResponse {
            id: 1,
            parent_id: None,
            path: "test".to_string(),
            name: "test".to_string(),
            folders: vec![],
            files: vec![],
            tags: vec![TagApi {
                id: Some(1),
                title: "tag1".to_string(),
            }],
        };
        assert_eq!(expected, get_folder(Some(1)).await.unwrap());
        cleanup();
    }
}

#[cfg(test)]
mod update_folder_tests {
    use rocket::tokio;

    use crate::model::error::folder_errors::UpdateFolderError;
    use crate::model::request::folder_requests::UpdateFolderRequest;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::TagApi;
    use crate::service::folder_service::{get_folder, update_folder};
    use crate::test::{
        cleanup, create_folder_db_entry, create_folder_disk, create_tag_folder, refresh_db,
    };

    #[tokio::test]
    async fn update_folder_adds_tags() {
        refresh_db();
        create_folder_db_entry("test", None);
        create_folder_disk("test");
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "test".to_string(),
            parent_id: None,
            tags: vec![TagApi {
                id: None,
                title: "tag1".to_string(),
            }],
        })
        .unwrap();
        let expected = FolderResponse {
            id: 1,
            parent_id: None,
            path: "test".to_string(),
            name: "test".to_string(),
            folders: vec![],
            files: vec![],
            tags: vec![TagApi {
                id: Some(1),
                title: "tag1".to_string(),
            }],
        };
        assert_eq!(expected, get_folder(Some(1)).await.unwrap());
        cleanup();
    }

    #[tokio::test]
    async fn update_folder_already_exists() {
        refresh_db();
        create_folder_db_entry("test", None);
        create_folder_db_entry("test2", None);
        let res = update_folder(&UpdateFolderRequest {
            id: 1,
            name: "test2".to_string(),
            parent_id: None,
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFolderError::AlreadyExists, res);
        let db_folder = get_folder(Some(1)).await.unwrap().name;
        assert_eq!("test", db_folder);
        cleanup();
    }

    #[tokio::test]
    async fn update_folder_removes_tags() {
        refresh_db();
        create_folder_db_entry("test", None);
        create_folder_disk("test");
        create_tag_folder("tag1", 1);
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "test".to_string(),
            parent_id: None,
            tags: vec![],
        })
        .unwrap();
        let expected = FolderResponse {
            id: 1,
            parent_id: None,
            path: "test".to_string(),
            name: "test".to_string(),
            folders: vec![],
            files: vec![],
            tags: vec![],
        };
        assert_eq!(expected, get_folder(Some(1)).await.unwrap());
        cleanup();
    }
}
