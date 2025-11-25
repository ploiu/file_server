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
    // get the folder before it's updated so that we can track changes
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
    // make sure we can't make a folder its own parent
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
        handle_folder_move_for_tags(folder.id, original_ancestors)?;
    }

    // Filter out implicit tags - only update explicit tags
    let explicit_tags: Vec<TaggedItemApi> = folder
        .tags
        .iter()
        .filter(|it| it.implicit_from.is_none())
        .cloned()
        .collect();
    tag_service::update_folder_tags(updated_folder.id.unwrap(), explicit_tags)
        .map_err(|_| UpdateFolderError::TagError)?;
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

/// Handles the removal of implicit tags from descendants when a folder is moved.
///
/// When a folder's parent changes, this function removes implicit tags from all
/// descendant files and folders that originated from the old ancestor chain.
///
/// ## Parameters
/// - `folder_id`: the id of the folder being moved
/// - `original_ancestors`: list of ancestor folder ids from before the move
///
/// ## Returns
/// - `Ok(())` if tags were successfully removed
/// - `Err(UpdateFolderError)` if there was a database error
fn handle_folder_move_for_tags(
    folder_id: u32,
    original_ancestors: Vec<u32>,
) -> Result<(), UpdateFolderError> {
    let con = repository::open_connection();

    // Get all descendant folders of the moved folder, so that we can remove stale tags
    let descendant_folders = match folder_repository::get_all_child_folder_ids(&[folder_id], &con) {
        Ok(folders) => folders,
        Err(e) => {
            log::error!(
                "Failed to retrieve descendant folders for folder {}. Error is {e:?}\n{}",
                folder_id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateFolderError::DbFailure);
        }
    };

    // Get all descendant files from the moved folder and its descendants
    let mut all_folder_ids = descendant_folders.clone();
    all_folder_ids.push(folder_id);
    let descendant_files: Vec<u32> = match folder_repository::get_child_files(&all_folder_ids, &con)
    {
        Ok(files) => files.into_iter().map(|f| f.id.unwrap()).collect(),
        Err(e) => {
            con.close().unwrap();
            log::error!(
                "Failed to retrieve descendant files for folder {}. Error is {e:?}\n{}",
                folder_id,
                Backtrace::force_capture()
            );
            return Err(UpdateFolderError::DbFailure);
        }
    };

    // Include the moved folder itself in the list of folders to remove implicit tags from
    let mut folders_to_update = descendant_folders.clone();
    folders_to_update.push(folder_id);

    // For each old ancestor, remove all implicit tags from that ancestor on the moved folder and its descendants
    if let Err(e) = tag_repository::batch_remove_implicit_tags(
        &descendant_files,
        &folders_to_update,
        &original_ancestors,
        &con,
    ) {
        con.close().unwrap();
        log::error!(
            "Failed to remove implicit tags after moving folder {folder_id}. Error is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(UpdateFolderError::TagError);
    }

    con.close().unwrap();
    Ok(())
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
    use crate::service::file_service;
    use crate::service::folder_service::{get_folder, update_folder};
    use crate::tags::service::{get_tags_on_file, update_file_tags};
    use crate::test::{
        cleanup, create_file_db_entry, create_folder_db_entry, create_folder_disk,
        create_tag_folder, imply_tag_on_folder, init_db_folder,
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
    fn moving_a_folder_to_root_removes_all_descendant_implicit_tags_from_original_ancestors() {
        init_db_folder();
        // Create folder structure: grandparent, parent, child
        create_folder_db_entry("grandparent", None);
        create_folder_db_entry("parent", Some(1));
        create_folder_db_entry("child", Some(2));
        create_file_db_entry("child_file", Some(3));
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
        let child_folder = get_folder(Some(3)).unwrap();
        assert_eq!(child_folder.tags.len(), 1);
        assert_eq!(child_folder.tags[0].title, "grandparent_tag");
        assert_eq!(child_folder.tags[0].implicit_from, Some(1));
        let child_file = file_service::get_file_metadata(1).unwrap();
        assert_eq!(child_file.tags.len(), 1);
        assert_eq!(child_file.tags[0].title, "grandparent_tag");
        assert_eq!(child_file.tags[0].implicit_from, Some(1));

        // Now move parent folder to new_parent - should remove grandparent's implicit tag from descendants
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: Some(4), // new_parent folder
            tags: vec![],
        })
        .unwrap();

        // Verify child no longer has implicit tag from grandparent
        let child = get_folder(Some(3)).unwrap();
        assert_eq!(child.tags.len(), 0);
        let child_file = file_service::get_file_metadata(1).unwrap();
        assert_eq!(child_file.tags.len(), 0);
        cleanup();
    }

    #[test]
    fn moving_folder_does_not_remove_tags_from_unaffected_folders() {
        init_db_folder();
        // Create folder structure:
        // grandparent (with tag)
        //   ├── parent
        //   │     └── child
        //   └── sibling (should keep grandparent tag even after parent moves)
        create_folder_db_entry("grandparent", None);
        create_folder_disk("grandparent");
        create_folder_db_entry("parent", Some(1));
        create_folder_disk("grandparent/parent");
        create_folder_db_entry("child", Some(2));
        create_folder_disk("grandparent/parent/child");
        create_folder_db_entry("sibling", Some(1));
        create_folder_disk("grandparent/sibling");

        // Add a tag to grandparent - should be implicated on parent, child, and sibling
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

        // Verify sibling and child both have implicit tag from grandparent
        let sibling = get_folder(Some(4)).unwrap();
        assert_eq!(sibling.tags.len(), 1);
        assert_eq!(sibling.tags[0].title, "grandparent_tag");
        assert_eq!(sibling.tags[0].implicit_from, Some(1));

        let child = get_folder(Some(3)).unwrap();
        assert_eq!(child.tags.len(), 1);

        // Move parent folder to root (simulates moving away from grandparent)
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: None, // moving to root
            tags: vec![],
        })
        .unwrap();

        // Verify sibling STILL has implicit tag from grandparent (unaffected by move)
        let sibling = get_folder(Some(4)).unwrap();
        assert_eq!(
            sibling.tags.len(),
            1,
            "Sibling should still have tag from grandparent"
        );
        assert_eq!(sibling.tags[0].title, "grandparent_tag");
        assert_eq!(sibling.tags[0].implicit_from, Some(1));

        // Verify child no longer has the tag (was moved out of grandparent)
        let child = get_folder(Some(3)).unwrap();
        assert_eq!(child.tags.len(), 0, "Child should have no tags after move");
        cleanup();
    }

    #[test]
    fn moving_folder_does_not_remove_explicit_tags_from_descendants() {
        init_db_folder();
        // Create folder structure: grandparent, parent (child of grandparent), child (child of parent)
        create_folder_db_entry("grandparent", None);
        create_folder_disk("grandparent");
        create_folder_db_entry("parent", Some(1));
        create_folder_disk("grandparent/parent");
        create_folder_db_entry("child", Some(2));
        create_folder_disk("grandparent/parent/child");

        create_file_db_entry("file.txt", Some(3));

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

        // Add an explicit tag to child folder
        update_folder(&UpdateFolderRequest {
            id: 3,
            name: "child".to_string(),
            parent_id: Some(2),
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "explicit_tag".to_string(),
                implicit_from: None,
            }],
        })
        .unwrap();

        // Add an explicit tag to the file
        update_file_tags(
            1,
            vec![TaggedItemApi {
                tag_id: None,
                title: "file_explicit_tag".to_string(),
                implicit_from: None,
            }],
        )
        .unwrap();

        // Verify child has both implicit and explicit tags before the move
        let child = get_folder(Some(3)).unwrap();
        // Child should have:
        // 1. grandparent_tag (implicit from grandparent id=1)
        // 2. explicit_tag (explicit on child id=3)
        // Note: When we set explicit tags on a folder, it propagates to descendants but not to the folder itself from parents
        assert_eq!(
            child.tags.len(),
            2,
            "Child should have 2 tags: implicit from grandparent and explicit"
        );

        // Verify file has implicit and explicit tags before the move
        let file_tags = get_tags_on_file(1).unwrap();
        // File should have multiple implicit tags from ancestors
        assert!(file_tags.len() >= 2, "File should have at least 2 tags");

        // Now move parent folder to root - should only remove grandparent's implicit tag
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: None, // moving to root
            tags: vec![],
        })
        .unwrap();

        // Verify child still has explicit tag but not implicit from grandparent
        let child = get_folder(Some(3)).unwrap();
        assert_eq!(
            child.tags.len(),
            1,
            "Child should have 1 tag after move: just the explicit tag"
        );
        assert_eq!(child.tags[0].title, "explicit_tag");
        assert_eq!(child.tags[0].implicit_from, None);

        // Verify file still has its explicit tag
        let file_tags = get_tags_on_file(1).unwrap();
        let has_file_explicit = file_tags
            .iter()
            .any(|t| t.title == "file_explicit_tag" && t.implicit_from.is_none());
        assert!(has_file_explicit, "File should have its explicit tag");

        // Verify file no longer has grandparent implicit tag
        let has_grandparent_tag = file_tags.iter().any(|t| t.title == "grandparent_tag");
        assert!(
            !has_grandparent_tag,
            "File should not have grandparent tag after move"
        );
        cleanup();
    }

    #[test]
    fn update_folder_implies_tags_to_descendant_files_in_nested_structure() {
        init_db_folder();
        create_folder_db_entry("parent", None);
        create_folder_disk("parent");
        create_folder_db_entry("child", Some(1));
        create_folder_disk("parent/child");

        create_file_db_entry("file_in_parent.txt", Some(1));
        create_file_db_entry("file_in_child.txt", Some(2));

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

        // Check both files have implicit tag
        let file1_tags = get_tags_on_file(1).unwrap();
        let file2_tags = get_tags_on_file(2).unwrap();

        let expected = TaggedItemApi {
            tag_id: Some(1),
            title: "tag1".to_string(),
            implicit_from: Some(1),
        };

        assert_eq!(file1_tags.len(), 1);
        assert_eq!(file1_tags[0], expected);
        assert_eq!(file2_tags.len(), 1);
        assert_eq!(file2_tags[0], expected);
        cleanup();
    }

    #[test]
    fn update_folder_removes_implicit_tags_from_descendant_files() {
        init_db_folder();
        create_folder_db_entry("parent", None);
        create_folder_disk("parent");
        create_file_db_entry("file.txt", Some(1));

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

        // Verify file has implicit tag
        let file_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file_tags.len(), 1);

        // Remove tag from parent
        update_folder(&UpdateFolderRequest {
            id: 1,
            name: "parent".to_string(),
            parent_id: None,
            tags: vec![],
        })
        .unwrap();

        // Verify file no longer has implicit tag
        let file_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file_tags.len(), 0);
        cleanup();
    }

    #[test]
    fn moving_a_folder_removes_all_descendant_file_implicit_tags_from_original_ancestors() {
        init_db_folder();
        // Create folder structure: grandparent, parent, child with files at each level
        create_folder_db_entry("grandparent", None);
        create_folder_db_entry("parent", Some(1));
        create_folder_db_entry("child", Some(2));
        create_folder_disk("grandparent/parent/child");

        create_file_db_entry("file_in_grandparent.txt", Some(1));
        create_file_db_entry("file_in_parent.txt", Some(2));
        create_file_db_entry("file_in_child.txt", Some(3));

        // Create another separate parent folder
        create_folder_db_entry("new_parent", None);
        create_folder_disk("new_parent");

        // Add a tag to grandparent - should be implicated on all descendant files
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

        // Verify all files have implicit tag from grandparent
        let file1_tags = get_tags_on_file(1).unwrap();
        let file2_tags = get_tags_on_file(2).unwrap();
        let file3_tags = get_tags_on_file(3).unwrap();

        assert_eq!(file1_tags.len(), 1);
        assert_eq!(file1_tags[0].title, "grandparent_tag");
        assert_eq!(file2_tags.len(), 1);
        assert_eq!(file2_tags[0].title, "grandparent_tag");
        assert_eq!(file3_tags.len(), 1);
        assert_eq!(file3_tags[0].title, "grandparent_tag");

        // Now move parent folder to new_parent - should remove grandparent's implicit tag from parent's descendants
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: Some(4), // new_parent folder
            tags: vec![],
        })
        .unwrap();

        // Verify files in parent and child no longer have implicit tag from grandparent
        let file2_tags = get_tags_on_file(2).unwrap();
        let file3_tags = get_tags_on_file(3).unwrap();
        assert_eq!(file2_tags.len(), 0);
        assert_eq!(file3_tags.len(), 0);

        // File in grandparent should still have the tag
        let file1_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file1_tags.len(), 1);
        assert_eq!(file1_tags[0].title, "grandparent_tag");
        cleanup();
    }

    #[test]
    fn moving_folder_does_not_remove_file_tags_from_unaffected_files() {
        init_db_folder();
        // Create folder structure with files
        create_folder_db_entry("grandparent", None);
        create_folder_disk("grandparent");
        create_folder_db_entry("parent", Some(1));
        create_folder_disk("grandparent/parent");
        create_folder_db_entry("child", Some(2));
        create_folder_disk("grandparent/parent/child");
        create_folder_db_entry("sibling", Some(1));
        create_folder_disk("grandparent/sibling");

        create_file_db_entry("file_in_child.txt", Some(3));
        create_file_db_entry("file_in_sibling.txt", Some(4));

        // Add a tag to grandparent
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

        // Verify both files have implicit tag from grandparent
        let file_child_tags = get_tags_on_file(1).unwrap();
        let file_sibling_tags = get_tags_on_file(2).unwrap();

        assert_eq!(file_child_tags.len(), 1);
        assert_eq!(file_sibling_tags.len(), 1);

        // Move parent folder to root
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: None, // moving to root
            tags: vec![],
        })
        .unwrap();

        // Verify sibling file STILL has implicit tag from grandparent (unaffected by move)
        let file_sibling_tags = get_tags_on_file(2).unwrap();
        assert_eq!(
            file_sibling_tags.len(),
            1,
            "Sibling file should still have tag from grandparent"
        );
        assert_eq!(file_sibling_tags[0].title, "grandparent_tag");
        assert_eq!(file_sibling_tags[0].implicit_from, Some(1));

        // Verify child file no longer has the tag (was moved out of grandparent)
        let file_child_tags = get_tags_on_file(1).unwrap();
        assert_eq!(
            file_child_tags.len(),
            0,
            "Child file should have no tags after move"
        );
        cleanup();
    }

    #[test]
    fn moving_folder_does_not_remove_explicit_tags_from_descendant_files() {
        init_db_folder();
        // Create folder structure: grandparent, parent, child
        create_folder_db_entry("grandparent", None);
        create_folder_disk("grandparent");
        create_folder_db_entry("parent", Some(1));
        create_folder_disk("grandparent/parent");
        create_folder_db_entry("child", Some(2));
        create_folder_disk("grandparent/parent/child");

        create_file_db_entry("file.txt", Some(3));

        // Add a tag to grandparent
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

        // Add an explicit tag to the file
        update_file_tags(
            1,
            vec![TaggedItemApi {
                tag_id: None,
                title: "file_explicit_tag".to_string(),
                implicit_from: None,
            }],
        )
        .unwrap();

        // Verify file has both implicit and explicit tags before the move
        let file_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file_tags.len(), 2, "File should have 2 tags before move");

        // Now move parent folder to root - should only remove grandparent's implicit tag
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "parent".to_string(),
            parent_id: None, // moving to root
            tags: vec![],
        })
        .unwrap();

        // Verify file still has its explicit tag
        let file_tags = get_tags_on_file(1).unwrap();
        assert_eq!(
            file_tags.len(),
            1,
            "File should have 1 tag after move: just the explicit tag"
        );
        assert_eq!(file_tags[0].title, "file_explicit_tag");
        assert_eq!(file_tags[0].implicit_from, None);
        cleanup();
    }

    #[test]
    fn update_folder_only_saves_explicit_tags_as_explicit() {
        init_db_folder();
        // Create folder hierarchy: parent1 -> child, and parent2
        create_folder_db_entry("parent1", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2
        create_folder_db_entry("parent2", None); // id 3
        create_folder_disk("parent1/child");
        create_folder_disk("parent2");
        create_tag_folder("implicitTag", 1);
        create_tag_folder("explicitTag", 2);
        imply_tag_on_folder(1, 2, 1);

        // Verify child has both explicit and implicit tags
        let child_before = get_folder(Some(2)).unwrap();
        assert_eq!(
            child_before.tags.len(),
            2,
            "Child should have 2 tags initially"
        );
        println!("Tags before update: {:?}", child_before.tags);

        // Now move child to parent2, passing BOTH tags in the request
        // The implicit tag should be marked as implicit in the request
        // If update_folder doesn't filter, it will try to save the implicit tag as explicit
        update_folder(&UpdateFolderRequest {
            id: 2,
            name: "child".to_string(),
            parent_id: Some(3), // Move to parent2
            tags: vec![
                // this tag should be kept
                TaggedItemApi {
                    tag_id: Some(2),
                    title: "explicitTag".to_string(),
                    implicit_from: None,
                },
                // this tag should be removed since it's implicit
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "implicitTag".to_string(),
                    implicit_from: Some(1),
                },
            ],
        })
        .unwrap();

        // Get the child folder again to check final state
        let child_after = get_folder(Some(2)).unwrap();

        // The child should ONLY have the explicit tag now
        // The implicit tag from parent1 should have been removed because we moved away from parent1
        // (The move logic removes implicit tags from old ancestors)
        assert_eq!(
            child_after.tags.len(),
            1,
            "Child should have only 1 tag after move"
        );

        // Verify it's the explicit tag
        assert_eq!(child_after.tags[0].title, "explicitTag");
        assert_eq!(
            child_after.tags[0].implicit_from, None,
            "Tag should be explicit"
        );

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
