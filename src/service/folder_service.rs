use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::Path;

use itertools::Itertools;
use regex::Regex;
use rusqlite::Connection;

use model::repository::Folder;

use crate::model::api::FileApi;
use crate::model::error::file_errors::{DeleteFileError, GetBulkPreviewError};
use crate::model::error::folder_errors::{
    CreateFolderError, DeleteFolderError, DownloadFolderError, GetChildFilesError, GetFolderError,
    UpdateFolderError,
};

use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
use crate::model::response::TaggedItemApi;
use crate::model::response::folder_responses::FolderResponse;
use crate::previews;
use crate::repository::{folder_repository, open_connection};
use crate::service::file_service;
use crate::service::file_service::{check_root_dir, file_dir};
use crate::tags::repository as tag_repository;
use crate::tags::service as tag_service;
use crate::tags::TagTypes;
use crate::{model, repository};

pub fn get_folder(id: Option<u32>) -> Result<FolderResponse, GetFolderError> {
    let db_id = if Some(0) == id || id.is_none() {
        None
    } else {
        id
    };
    let folder = get_folder_by_id(db_id)?;
    let mut folder: FolderResponse = folder.into();
    let con: Connection = repository::open_connection();
    let child_folders = match folder_repository::get_child_folders(db_id, &con) {
        Ok(f) => f,
        Err(e) => {
            log::error!(
                "Failed to pull child folders from database! Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(GetFolderError::DbFailure);
        }
    };
    let mut converted_folders: Vec<FolderResponse> = Vec::new();
    for child in child_folders {
        let tags: Vec<TaggedItemApi> =
            match tag_repository::get_all_tags_for_folder(child.id.unwrap_or(0), &con) {
                Ok(t) => t.into_iter().map_into().collect(),
                Err(e) => {
                    log::error!(
                        "Failed to retrieve tags for folder. Exception is {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    con.close().unwrap();
                    return Err(GetFolderError::TagError);
                }
            };
        let mut converted: FolderResponse = child.into();
        converted += tags;
        converted_folders.push(converted);
    }
    let tag_db_id = id.unwrap_or_default();
    let tags = match tag_service::get_tags_on_folder(tag_db_id) {
        Ok(t) => t,
        Err(_) => {
            con.close().unwrap();
            return Err(GetFolderError::TagError);
        }
    };
    folder += get_files_for_folder(db_id, &con).unwrap();
    con.close().unwrap();
    folder += converted_folders;
    folder += tags;
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
                Ok(_) => Ok(f.into()),
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
        Err(GetFolderError::NotFound) => return Err(UpdateFolderError::NotFound),
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
    
    // Check if parent_id is changing - if so, we need to remove old implicit tags from descendants
    let parent_id_changed = original_folder.parent_id != db_folder.parent_id;
    let original_ancestors = if parent_id_changed {
        let con = repository::open_connection();
        let ancestors = match folder_repository::get_ancestor_folders_with_id(folder.id, &con) {
            Ok(a) => a,
            Err(e) => {
                log::error!(
                    "Failed to retrieve ancestor folders for folder {}. Error is {e:?}\n{}",
                    folder.id,
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(UpdateFolderError::DbFailure);
            }
        };
        con.close().unwrap();
        ancestors
    } else {
        Vec::new()
    };
    
    let updated_folder = update_folder_internal(&db_folder)?;
    // if we can't rename the folder, then we have problems
    if let Err(e) = fs::rename(
        format!("{}/{}", file_dir(), original_folder.name),
        &updated_folder.name,
    ) {
        log::error!(
            "Failed to move folder! Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(UpdateFolderError::FileSystemFailure);
    }
    // updated folder name will be a path, so we need to get just the folder name
    let split_name = String::from(&updated_folder.name);
    let mut split_name = split_name.split('/');
    let name = String::from(
        split_name
            .next_back()
            .unwrap_or(updated_folder.name.as_str()),
    );
    // If the parent changed, remove implicit tags from descendants that came from old ancestors
    if parent_id_changed {
        let con = repository::open_connection();
        
        // Get all descendant folders
        let descendant_folders = match folder_repository::get_all_child_folder_ids(&[folder.id], &con) {
            Ok(folders) => folders,
            Err(e) => {
                log::error!(
                    "Failed to retrieve descendant folders for folder {}. Error is {e:?}\n{}",
                    folder.id,
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(UpdateFolderError::DbFailure);
            }
        };
        
        // Get all descendant files from the folder and its descendants
        let mut all_folder_ids = descendant_folders.clone();
        all_folder_ids.push(folder.id);
        match folder_repository::get_child_files(&all_folder_ids, &con) {
            Ok(_files) => { /* retrieved successfully, we just needed to check */ }
            Err(e) => {
                log::error!(
                    "Failed to retrieve descendant files for folder {}. Error is {e:?}\n{}",
                    folder.id,
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(UpdateFolderError::DbFailure);
            }
        };
        
        // For each old ancestor, get its tags and remove them from descendants
        for ancestor_id in original_ancestors {
            let ancestor_tags = match tag_repository::get_tags_for_folder(ancestor_id, TagTypes::Explicit, &con) {
                Ok(tags) => tags,
                Err(e) => {
                    log::error!(
                        "Failed to retrieve tags for ancestor folder {}. Error is {e:?}\n{}",
                        ancestor_id,
                        Backtrace::force_capture()
                    );
                    con.close().unwrap();
                    return Err(UpdateFolderError::TagError);
                }
            };
            
            // Remove implicit tags from descendant files
            for tag in &ancestor_tags {
                if let Err(e) = tag_repository::remove_implicit_tag_from_files(tag.tag_id, ancestor_id, &con) {
                    log::error!(
                        "Failed to remove implicit tag from files. Error is {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    con.close().unwrap();
                    return Err(UpdateFolderError::TagError);
                }
            }
            
            // Remove implicit tags from descendant folders
            for tag in &ancestor_tags {
                if let Err(e) = tag_repository::remove_implicit_tags_from_folders(tag.tag_id, ancestor_id, &con) {
                    log::error!(
                        "Failed to remove implicit tag from folders. Error is {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    con.close().unwrap();
                    return Err(UpdateFolderError::TagError);
                }
            }
        }
        
        con.close().unwrap();
    }
    
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
    // no id in the database if it's 0. 0 || None -> None, else keep original value of id
    // TODO change to is_none_or once it's no longer unstable (https://doc.rust-lang.org/stable/std/option/enum.Option.html#method.is_none_or)
    let db_id = id.filter(|&it| it != 0);
    let res = folder_repository::get_by_id(db_id, &con);
    con.close().unwrap();
    res.is_ok()
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
        log::error!(
            "Failed to recursively delete folder from disk! Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(DeleteFolderError::FileSystemError);
    };
    Ok(())
}

#[deprecated(note = "prefer to use the streaming version in preview_service")]
pub async fn get_file_previews_for_folder(
    id: u32,
) -> Result<HashMap<u32, Vec<u8>>, GetBulkPreviewError> {
    let con: Connection = open_connection();
    let ids: Vec<u32> = if id == 0 { vec![] } else { vec![id] };
    let file_ids: Vec<u32> = match folder_repository::get_child_files(&ids, &con) {
        Ok(res) => res,
        Err(e) if e != rusqlite::Error::QueryReturnedNoRows => {
            con.close().unwrap();
            log::error!(
                "Failed to query files for folder {id}. Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(GetBulkPreviewError::Db);
        }
        Err(_e) => vec![],
    }
    .into_iter()
    .map(|it| it.id.unwrap())
    .collect();
    let mut map: HashMap<u32, Vec<u8>> = HashMap::new();
    for id in file_ids {
        let preview = match previews::get_file_preview(id).await {
            Ok(p) => p,
            Err(_) => continue,
        };
        map.insert(id, preview);
    }
    con.close().unwrap();
    Ok(map)
}

/// compresses the folder with the passed id in tar format, stores it in the system temp directory, and returns the resulting file.
/// If the id is 0, this function fails if the folder isn't found or if the folder is root. While technically possible, the root folder shouldn't
/// be downloaded in its entirety - that just seems suspicious. Regular backups should be made outside of the api, and I don't want this endpoint to be
/// used in place of properly backup up your stuff
pub fn download_folder(id: u32) -> Result<File, DownloadFolderError> {
    if id == 0 {
        return Err(DownloadFolderError::RootFolder);
    }
    let folder = get_folder(Some(id)).map_err(|e| {
        log::error!(
            "Failed to retrieve folder with id {id} from the database; {e:?}\n{}",
            Backtrace::force_capture()
        );
        DownloadFolderError::NotFound
    })?;
    let temp_dir = std::env::temp_dir();
    // nano id used here to ensure file names are unique if the same file is downloaded multiple times
    let tarchive_dir = format!(
        "{}/{}-{}.tar",
        temp_dir.display(),
        folder.name,
        nanoid::nanoid!()
    );
    // so we have to actually create the tar archive file first before passing it to the builder
    let tarchive = File::create(tarchive_dir.clone()).map_err(|e| {
        log::error!(
            "Failed to create tar archive for {tarchive_dir}; {e:?}\n{}",
            Backtrace::force_capture()
        );
        DownloadFolderError::Tar
    })?;
    let mut tarchive_builder = tar::Builder::new(tarchive);
    if let Err(e) = tarchive_builder.append_dir_all("", format!("{}/{}", file_dir(), folder.path)) {
        log::error!(
            "Failed to tarchive {}/{}; {e:?}\n{}",
            file_dir(),
            folder.path,
            Backtrace::force_capture()
        );
        return Err(DownloadFolderError::Tar);
    };
    if let Err(e) = tarchive_builder.finish() {
        log::error!(
            "Failed to close tarchive {tarchive_dir}; {e:?}\n{}",
            Backtrace::force_capture()
        );
    }
    File::open(tarchive_dir.clone()).map_err(|_| DownloadFolderError::NotFound)
}

fn get_folder_by_id(id: Option<u32>) -> Result<Folder, GetFolderError> {
    // the client can pass 0 for the folder id, in which case it needs to be translated to None for the database
    let db_folder = if let Some(0) = id { None } else { id };
    let con = repository::open_connection();
    let result = match folder_repository::get_by_id(db_folder, &con) {
        Ok(folder) => Ok(folder),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(GetFolderError::NotFound),
        Err(err) => {
            log::error!(
                "Failed to pull folder info from database! Nested exception is {err:?}\n{}",
                Backtrace::force_capture()
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
                    if new_folder_path == child.name {
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
            log::error!(
                "Error trying to save folder!\nException is {e:?}\n{}",
                Backtrace::force_capture()
            );
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
    if !folder_exists(folder.id) {
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
        log::error!(
            "Failed to update folder in database. Nested exception is {:?}\n{}",
            update.unwrap_err(),
            Backtrace::force_capture()
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
    name: &str,
    parent_id: Option<u32>,
    con: &Connection,
) -> Result<Option<Folder>, rusqlite::Error> {
    let matching_folder = folder_repository::get_child_folders(parent_id, con)?
        .iter()
        .map(|folder| Folder {
            id: folder.id,
            parent_id: folder.parent_id,
            name: String::from(folder.name.to_lowercase().split('/').next_back().unwrap()),
        })
        .find(|folder| folder.name == name.to_lowercase().split('/').next_back().unwrap());
    Ok(matching_folder)
}

fn does_file_exist(
    name: &str,
    folder_id: Option<u32>,
    con: &Connection,
) -> Result<bool, rusqlite::Error> {
    let unwrapped_id: Vec<u32> = folder_id.map(|it| vec![it]).unwrap_or_default();
    let matching_file = folder_repository::get_child_files(&unwrapped_id, con)?
        .iter()
        .find(|file| file.name == name.to_lowercase())
        .cloned();
    Ok(matching_file.is_some())
}

/// checks if the new_parent_id being passed matches any id of any sub child of the passed folder_id
fn is_attempt_move_to_sub_child(
    folder_id: &u32,
    new_parent_id: &u32,
    con: &Connection,
) -> Result<bool, UpdateFolderError> {
    match folder_repository::get_all_child_folder_ids(&[*folder_id], con) {
        Ok(ids) => {
            if ids.contains(new_parent_id) {
                Err(UpdateFolderError::NotAllowed)
            } else {
                Ok(true)
            }
        }
        _ => Err(UpdateFolderError::DbFailure),
    }
}

/// returns the top-level files for the passed folder
fn get_files_for_folder(
    id: Option<u32>,
    con: &Connection,
) -> Result<Vec<FileApi>, GetChildFilesError> {
    // now we can retrieve all the file records in this folder
    let unwrapped_id = id.map(|it| vec![it]).unwrap_or_default();
    let child_files = match folder_repository::get_child_files(&unwrapped_id, con) {
        Ok(files) => files,
        Err(e) => {
            log::error!(
                "Failed to query database for child files. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(GetChildFilesError::DbFailure);
        }
    };
    let file_ids: Vec<u32> = child_files
        .iter()
        .map(|f| f.id.expect("files pulled from database didn't have ID!"))
        .collect();
    let file_tags = match tag_repository::get_all_tags_for_files(file_ids, con) {
        Ok(res) => res,
        Err(e) => {
            log::error!(
                "Failed to get tags on file {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(GetChildFilesError::TagError);
        }
    };
    let mut result: Vec<FileApi> = Vec::new();
    for file in child_files {
        let tags = if file_tags.contains_key(&file.id.unwrap()) {
            file_tags.get(&file.id.unwrap()).unwrap().clone()
        } else {
            Vec::new()
        };
        let tags: Vec<TaggedItemApi> = tags.iter().cloned().map_into().collect();
        result.push(FileApi::from_with_tags(file, tags));
    }
    Ok(result)
}

/// the main body of `delete_folder`. Takes a connection so that we're not creating a connection on every stack frame
fn delete_folder_recursively(id: u32, con: &Connection) -> Result<Folder, DeleteFolderError> {
    let folder = folder_repository::get_by_id(Some(id), con).map_err(|e| {
        log::error!(
            "Failed to recursively delete folder. Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        if e == rusqlite::Error::QueryReturnedNoRows {
            DeleteFolderError::FolderNotFound
        } else {
            DeleteFolderError::DbFailure
        }
    })?;
    // now that we have the folder, we can delete all the files for that folder
    let files =
        folder_repository::get_child_files(&[id], con).map_err(|_| DeleteFolderError::DbFailure)?;
    for file in files.iter() {
        match file_service::delete_file_by_id_with_connection(file.id.unwrap(), con) {
            Err(DeleteFileError::NotFound) => {}
            Err(_) => return Err(DeleteFolderError::DbFailure),
            Ok(_) => { /*no op - file was removed properly*/ }
        };
    }
    // now that we've deleted all files, we can try with all folders
    let sub_folders = folder_repository::get_child_folders(Some(id), con)
        .map_err(|_| DeleteFolderError::DbFailure)?;
    for sub_folder in sub_folders.iter() {
        delete_folder_recursively(sub_folder.id.unwrap(), con)?;
    }
    // now that we've deleted everything beneath it, delete the requested folder from the repository
    if let Err(e) = folder_repository::delete_folder(id, con) {
        log::error!(
            "Failed to delete root folder in recursive folder delete. Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(DeleteFolderError::DbFailure);
    };
    Ok(folder)
}

#[cfg(test)]
mod get_folder_tests {
    use crate::model::error::folder_errors::GetFolderError;
    use crate::model::response::TaggedItemApi;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::service::folder_service::get_folder;
    use crate::test::{cleanup, create_folder_db_entry, create_tag_folder, init_db_folder};

    #[test]
    fn get_folder_works() {
        init_db_folder();
        create_folder_db_entry("test", None);
        let folder = get_folder(Some(1)).unwrap();
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

    #[test]
    fn get_folder_not_found() {
        init_db_folder();
        let err = get_folder(Some(1)).unwrap_err();
        assert_eq!(GetFolderError::NotFound, err);
        cleanup();
    }

    #[test]
    fn get_folder_retrieves_tags() {
        init_db_folder();
        create_folder_db_entry("test", None);
        create_tag_folder("tag1", 1);
        let expected = FolderResponse {
            id: 1,
            parent_id: None,
            path: "test".to_string(),
            name: "test".to_string(),
            folders: vec![],
            files: vec![],
            tags: vec![TaggedItemApi {
                tag_id: Some(1),
                title: "tag1".to_string(),
                implicit_from: None,
            }],
        };
        assert_eq!(expected, get_folder(Some(1)).unwrap());
        cleanup();
    }
}

#[cfg(test)]
mod update_folder_tests {
    use crate::model::error::folder_errors::UpdateFolderError;
    use crate::model::request::folder_requests::UpdateFolderRequest;
    use crate::model::response::TaggedItemApi;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::service::folder_service::{get_folder, update_folder};
    use crate::test::{
        cleanup, create_folder_db_entry, create_folder_disk, create_tag_folder, init_db_folder,
    };

    #[test]
    fn update_folder_adds_tags() {
        init_db_folder();
        create_folder_db_entry("test", None);
        create_folder_disk("test");
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "test".to_string(),
            parent_id: None,
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "tag1".to_string(),
                implicit_from: None,
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
            tags: vec![TaggedItemApi {
                tag_id: Some(1),
                title: "tag1".to_string(),
                implicit_from: None,
            }],
        };
        assert_eq!(expected, get_folder(Some(1)).unwrap());
        cleanup();
    }

    #[test]
    fn update_folder_already_exists() {
        init_db_folder();
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
        let db_folder = get_folder(Some(1)).unwrap().name;
        assert_eq!("test", db_folder);
        cleanup();
    }

    #[test]
    fn update_folder_removes_tags() {
        init_db_folder();
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
        assert_eq!(expected, get_folder(Some(1)).unwrap());
        cleanup();
    }

    #[test]
    fn update_folder_implies_tags_to_descendant_folders() {
        init_db_folder();
        create_folder_db_entry("parent", None);
        create_folder_disk("parent");
        create_folder_db_entry("child", Some(1));
        create_folder_disk("parent/child");

        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "parent".to_string(),
            parent_id: None,
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "tag1".to_string(),
                implicit_from: None,
            }],
        })
        .unwrap();

        // Check child folder has implicit tag
        let child = get_folder(Some(2)).unwrap();
        let expected = TaggedItemApi {
            tag_id: Some(1),
            title: "tag1".to_string(),
            implicit_from: Some(1),
        };
        assert_eq!(child.tags.len(), 1);
        assert_eq!(child.tags[0], expected);
        cleanup();
    }

    #[test]
    fn update_folder_implies_tags_to_descendant_files() {
        init_db_folder();
        create_folder_db_entry("parent", None);
        create_folder_disk("parent");

        use crate::test::create_file_db_entry;
        create_file_db_entry("file.txt", Some(1));

        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "parent".to_string(),
            parent_id: None,
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "tag1".to_string(),
                implicit_from: None,
            }],
        })
        .unwrap();

        // Check file has implicit tag
        use crate::tags::service::get_tags_on_file;
        let file_tags = get_tags_on_file(1).unwrap();
        let expected = TaggedItemApi {
            tag_id: Some(1),
            title: "tag1".to_string(),
            implicit_from: Some(1),
        };
        assert_eq!(file_tags.len(), 1);
        assert_eq!(file_tags[0], expected);
        cleanup();
    }

    #[test]
    fn update_folder_removes_implicit_tags_from_descendants() {
        init_db_folder();
        create_folder_db_entry("parent", None);
        create_folder_disk("parent");
        create_folder_db_entry("child", Some(1));
        create_folder_disk("parent/child");

        // Add tag and propagate
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "parent".to_string(),
            parent_id: None,
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "tag1".to_string(),
                implicit_from: None,
            }],
        })
        .unwrap();

        // Verify child has implicit tag
        let child = get_folder(Some(2)).unwrap();
        assert_eq!(child.tags.len(), 1);

        // Remove tag from parent
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "parent".to_string(),
            parent_id: None,
            tags: vec![],
        })
        .unwrap();

        // Verify child no longer has implicit tag
        let child = get_folder(Some(2)).unwrap();
        assert_eq!(child.tags.len(), 0);
        cleanup();
    }

    #[test]
    fn moving_a_folder_to_another_folder_recalculates_descendant_implicit_tags() {
        init_db_folder();
        // Create folder structure: grandparent, parent (child of grandparent), child (child of parent)
        create_folder_db_entry("grandparent", None);
        create_folder_disk("grandparent");
        create_folder_db_entry("parent", Some(1));
        create_folder_disk("grandparent/parent");
        create_folder_db_entry("child", Some(2));
        create_folder_disk("grandparent/parent/child");
        
        // Create another separate parent folder
        create_folder_db_entry("new_parent", None);
        create_folder_disk("new_parent");
        
        // Add a tag to grandparent - should be implicated on parent and child
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "grandparent".to_string(),
            parent_id: None,
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "grandparent_tag".to_string(),
                implicit_from: None,
            }],
        })
        .unwrap();
        
        // Verify child has implicit tag from grandparent
        let child = get_folder(Some(3)).unwrap();
        assert_eq!(child.tags.len(), 1);
        assert_eq!(child.tags[0].title, "grandparent_tag");
        assert_eq!(child.tags[0].implicit_from, Some(1));
        
        // Now move parent folder to new_parent - should remove grandparent's implicit tag from descendants
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: Some(4),  // new_parent folder
            tags: vec![],
        })
        .unwrap();
        
        // Verify child no longer has implicit tag from grandparent
        let child = get_folder(Some(3)).unwrap();
        assert_eq!(child.tags.len(), 0);
        cleanup();
    }

    #[test]
    fn moving_a_folder_to_root_removes_all_descendant_implicit_tags_from_original_ancestors() {
        init_db_folder();
        // Create folder structure: grandparent, parent (child of grandparent), child (child of parent)
        create_folder_db_entry("grandparent", None);
        create_folder_disk("grandparent");
        create_folder_db_entry("parent", Some(1));
        create_folder_disk("grandparent/parent");
        create_folder_db_entry("child", Some(2));
        create_folder_disk("grandparent/parent/child");
        
        // Add a tag to grandparent - should be implicated on parent and child
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "grandparent".to_string(),
            parent_id: None,
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "grandparent_tag".to_string(),
                implicit_from: None,
            }],
        })
        .unwrap();
        
        // Verify child has implicit tag from grandparent
        let child = get_folder(Some(3)).unwrap();
        assert_eq!(child.tags.len(), 1);
        assert_eq!(child.tags[0].title, "grandparent_tag");
        assert_eq!(child.tags[0].implicit_from, Some(1));
        
        // Now move parent folder to root - should remove grandparent's implicit tag from descendants
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: None,  // moving to root
            tags: vec![],
        })
        .unwrap();
        
        // Verify child no longer has implicit tag from grandparent
        let child = get_folder(Some(3)).unwrap();
        assert_eq!(child.tags.len(), 0);
        cleanup();
    }
}

#[cfg(test)]
mod download_folder_tests {
    use crate::{
        service::folder_service::download_folder,
        test::{cleanup, create_folder_db_entry, create_folder_disk, init_db_folder},
    };

    #[test]
    fn works() {
        init_db_folder();
        create_folder_disk("test/top/middle/bottom");
        create_folder_db_entry("test", None);
        create_folder_db_entry("top", Some(1));
        create_folder_db_entry("middle", Some(2));
        create_folder_db_entry("bottom", Some(3));
        assert!(download_folder(2).is_ok());
        cleanup();
    }
}
