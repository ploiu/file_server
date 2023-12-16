use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hash;
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
use crate::model::response::TagApi;
use crate::repository::{folder_repository, open_connection};
use crate::service::file_service::{check_root_dir, file_dir};
use crate::service::{file_service, tag_service};
use crate::{model, repository};

pub fn get_folder(id: Option<u32>) -> Result<FolderResponse, GetFolderError> {
    let db_id = if Some(0) == id || id.is_none() {
        None
    } else {
        id
    };
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
    let tag_db_id = if let Some(id_res) = id { id_res } else { 0 };
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
    let split_name = split_name.split('/');
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
        eprintln!(
            "Failed to recursively delete folder from disk! Nested exception is: \n {:?}",
            e
        );
        return Err(DeleteFolderError::FileSystemError);
    };
    Ok(())
}

pub fn get_folders_by_any_tag(
    tags: &HashSet<String>,
) -> Result<HashSet<FolderResponse>, GetFolderError> {
    let con: Connection = open_connection();
    let folders = match folder_repository::get_folders_by_any_tag(tags, &con) {
        Ok(f) => f,
        Err(e) => {
            con.close().unwrap();
            log::error!("Failed to pull folders by any tag. Exception is {e}");
            return Err(GetFolderError::DbFailure);
        }
    };
    con.close().unwrap();
    let mut converted_folders: HashSet<FolderResponse> = HashSet::with_capacity(folders.len());
    for folder in folders {
        let tags = match tag_service::get_tags_on_folder(folder.id.unwrap()) {
            Ok(t) => t,
            Err(_) => return Err(GetFolderError::TagError),
        };
        converted_folders.insert(FolderResponse {
            id: folder.id.unwrap(),
            parent_id: folder.parent_id,
            name: folder.name,
            path: "no path".to_string(),
            folders: Vec::new(),
            files: Vec::new(),
            tags,
        });
    }
    Ok(converted_folders)
}

/// will reduce a list of folders down to the first one that has all the tags
/// the folders passed must be all the folders retrieved in [folder_service::get_folders_by_any_tag]
pub fn reduce_folders_by_tag(
    folders: &HashSet<FolderResponse>,
    tags: &HashSet<String>,
) -> Result<HashSet<FolderResponse>, GetFolderError> {
    // an index of the contents of condensed, to easily look up entries.
    let mut condensed_list: HashMap<u32, FolderResponse> = HashMap::new();
    // this will never change, because sometimes we need to pull folder info no longer in the condensed list if we're a child
    let mut input_index: HashMap<u32, &FolderResponse> = HashMap::new();
    for folder in folders {
        // I don't like having to clone all the folders, but with just references the compiler complains about reference lifetimes
        condensed_list.insert(folder.id, folder.clone());
        input_index.insert(folder.id, folder);
    }
    let con: Connection = open_connection();
    for (folder_id, folder) in input_index.iter() {
        // 1. skip if we're not in condensed_list; we were removed in an earlier step
        if !condensed_list.contains_key(folder_id) {
            continue;
        }
        // 2. get all parent folder IDs, take their tags for ourself, and remove those parents from condensed_list
        let mut our_tag_titles = folder
            .tags
            .iter()
            .map(|t| t.title.clone())
            .collect::<HashSet<String>>();
        let parents = match folder_repository::get_parent_folders_by_tag(*folder_id, &tags, &con) {
            Ok(p) => p,
            Err(e) => {
                con.close().unwrap();
                log::error!("Failed to pull parent folders. Exception is {e}");
                return Err(GetFolderError::DbFailure);
            }
        };
        for (parent_id, parent_tags) in parents {
            // if the parent has all of our tags, we need to remove ourself (and our children)
            if contains_all(&parent_tags, tags) {
                condensed_list.remove(folder_id);
                // this will tell `give_children_tags` that we already have all the tags (which we do because our parent does), so all the children get removed
                our_tag_titles = parent_tags;
                break;
            }
            parent_tags.into_iter().for_each(|t| {
                our_tag_titles.insert(t);
            });
            condensed_list.remove(&parent_id);
        }
        // 3. + 4. get all children folder IDs, give them our tags, and remove ourself from condensed_list if we have children in condensed_list
        if let Err(e) =
            give_children_tags(&mut condensed_list, &con, *folder_id, &our_tag_titles, tags)
        {
            con.close().unwrap();
            return Err(e);
        };
        // 5. remove ourself from condensed_list if we do not have all tags
        if !contains_all(&our_tag_titles, tags) {
            condensed_list.remove(folder_id);
        }
    }
    con.close().unwrap();
    let copied: HashSet<FolderResponse> = condensed_list.into_values().collect();
    Ok(copied)
}

// private functions
/// used as part of [reduce_folders_by_tag];
/// handles giving all children our tags, and removing ourself if we have any children with tags we don't have
fn give_children_tags(
    condensed_list: &mut HashMap<u32, FolderResponse>,
    con: &Connection,
    folder_id: u32,
    our_tag_titles: &HashSet<String>,
    tags: &HashSet<String>,
) -> Result<(), GetFolderError> {
    let all_child_folders_ids = match folder_repository::get_all_child_folder_ids(&[folder_id], con)
    {
        Ok(ids) => ids
            .into_iter()
            .filter(|id| condensed_list.contains_key(id))
            .collect::<Vec<u32>>(),
        Err(e) => {
            log::error!(
                "Failed to retrieve all child folder IDs for {folder_id}. Exception is {e}"
            );
            return Err(GetFolderError::DbFailure);
        }
    };
    // if we have all of the tags, remove all our children because they're not the highest
    if contains_all(our_tag_titles, tags) {
        for id in all_child_folders_ids.iter() {
            condensed_list.remove(id);
        }
        return Ok(());
    }
    for id in all_child_folders_ids.iter() {
        let matching_folder = condensed_list.get_mut(id).unwrap();
        let matching_folder_tags = matching_folder.tags.clone();
        let combined_tag_titles = matching_folder_tags
            .iter()
            .map(|t| t.title.clone())
            .chain(our_tag_titles.clone().into_iter());
        let combined_tags = matching_folder_tags
            .iter()
            .map(|t| &t.title)
            .chain(our_tag_titles.iter())
            .map(|title| TagApi {
                id: None,
                title: title.clone(),
            })
            .collect::<Vec<TagApi>>();
        *matching_folder = FolderResponse {
            id: matching_folder.id,
            parent_id: matching_folder.parent_id,
            path: matching_folder.path.clone(),
            name: matching_folder.name.clone(),
            folders: vec![],
            files: vec![],
            tags: combined_tags,
        };
        // 4. remove all children who only have the same tags as us, because they're not the earliest with all tags (or they will never have all tags)
        if HashSet::from_iter(combined_tag_titles) == *our_tag_titles {
            condensed_list.remove(id);
        }
    }
    if !all_child_folders_ids.is_empty() {
        condensed_list.remove(&folder_id);
    }
    Ok(())
}

fn get_folder_by_id(id: Option<u32>) -> Result<Folder, GetFolderError> {
    // the client can pass 0 for the folder id, in which case it needs to be translated to None for the database
    let db_folder = if let Some(0) = id { None } else { id };
    let con = repository::open_connection();
    let result = match folder_repository::get_by_id(db_folder, &con) {
        Ok(folder) => Ok(folder),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(GetFolderError::NotFound),
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
    name: &str,
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
    name: &str,
    folder_id: Option<u32>,
    con: &Connection,
) -> Result<bool, rusqlite::Error> {
    let unwrapped_id: Vec<u32> = folder_id.map(|it| vec![it]).unwrap_or_default();
    let matching_file = folder_repository::get_child_files(unwrapped_id, con)?
        .iter()
        // this is required because apparently the file is dropped immediately when it's used...
        .map(|file| FileRecord {
            id: file.id,
            name: String::from(&file.name),
            parent_id: folder_id,
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
    let unwrapped_id = id.map(|it| vec![it]).unwrap_or_default();
    let child_files = match folder_repository::get_child_files(unwrapped_id, &con) {
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
    let folder = folder_repository::get_by_id(Some(id), con).map_err(|e| {
        eprintln!(
            "Failed to recursively delete folder. Nested exception is {:?}",
            e
        );
        if e == rusqlite::Error::QueryReturnedNoRows {
            DeleteFolderError::FolderNotFound
        } else {
            DeleteFolderError::DbFailure
        }
    })?;
    // now that we have the folder, we can delete all the files for that folder
    let files =
        folder_repository::get_child_files([id], con).map_err(|_| DeleteFolderError::DbFailure)?;
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
        eprintln!(
            "Failed to delete root folder in recursive folder delete. Nested exception is: \n {:?}",
            e
        );
        return Err(DeleteFolderError::DbFailure);
    };
    Ok(folder)
}

/// checks if the first hash set contains all the items in the second hash set
fn contains_all<T: Eq + Hash + Clone>(first: &HashSet<T>, second: &HashSet<T>) -> bool {
    let intersection: HashSet<T> = first.intersection(second).cloned().collect();
    &intersection == second
}

#[cfg(test)]
mod get_folder_tests {
    use crate::model::error::folder_errors::GetFolderError;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::TagApi;
    use crate::service::folder_service::get_folder;
    use crate::test::{cleanup, create_folder_db_entry, create_tag_folder, refresh_db};

    #[test]
    fn get_folder_works() {
        refresh_db();
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
        refresh_db();
        let err = get_folder(Some(1)).unwrap_err();
        assert_eq!(GetFolderError::NotFound, err);
        cleanup();
    }

    #[test]
    fn get_folder_retrieves_tags() {
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
        assert_eq!(expected, get_folder(Some(1)).unwrap());
        cleanup();
    }
}

#[cfg(test)]
mod update_folder_tests {
    use crate::model::error::folder_errors::UpdateFolderError;
    use crate::model::request::folder_requests::UpdateFolderRequest;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::TagApi;
    use crate::service::folder_service::{get_folder, update_folder};
    use crate::test::{
        cleanup, create_folder_db_entry, create_folder_disk, create_tag_folder, refresh_db,
    };

    #[test]
    fn update_folder_adds_tags() {
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
        assert_eq!(expected, get_folder(Some(1)).unwrap());
        cleanup();
    }

    #[test]
    fn update_folder_already_exists() {
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
        let db_folder = get_folder(Some(1)).unwrap().name;
        assert_eq!("test", db_folder);
        cleanup();
    }

    #[test]
    fn update_folder_removes_tags() {
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
        assert_eq!(expected, get_folder(Some(1)).unwrap());
        cleanup();
    }
}

#[cfg(test)]
mod reduce_folders_by_tag_tests {
    use std::collections::HashSet;

    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::TagApi;
    use crate::service::folder_service::reduce_folders_by_tag;
    use crate::test::{
        cleanup, create_file_db_entry, create_folder_db_entry, create_tag_folder,
        create_tag_folders, refresh_db,
    };

    #[test]
    fn reduce_folders_by_tag_works() {
        refresh_db();
        create_folder_db_entry("A", None); // 1
        create_folder_db_entry("AB", Some(1)); // 2
        create_folder_db_entry("ABB", Some(1)); // 3
        create_folder_db_entry("AC", Some(2)); // 4
        create_folder_db_entry("Dummy5", None); // 5
        create_folder_db_entry("E", None); // 6
        create_folder_db_entry("EB", Some(6)); // 7
        create_folder_db_entry("EC", Some(7)); // 8
        create_folder_db_entry("Dummy9", None); // 9
        create_folder_db_entry("Dummy10", None); // 10
        create_folder_db_entry("Dummy11", None); // 11
        create_folder_db_entry("Dummy12", None); // 12
        create_folder_db_entry("Dummy13", None); // 13
        create_folder_db_entry("XA", None); // 14
        create_folder_db_entry("X", Some(14)); // 15
        create_folder_db_entry("Y", None); // 16
        create_folder_db_entry("Z", Some(16)); // 17
        create_tag_folders("tag1", vec![6, 16, 17, 2, 15, 14, 1]);
        create_tag_folders("tag3", vec![4, 15, 8, 3]);
        create_tag_folders("tag2", vec![2, 15, 3, 7]);
        let folders = HashSet::from([
            FolderResponse {
                id: 6,
                parent_id: None,
                path: "".to_string(),
                name: "E".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag1".to_string(),
                }],
            },
            FolderResponse {
                id: 16,
                parent_id: None,
                path: "".to_string(),
                name: "Y".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag1".to_string(),
                }],
            },
            FolderResponse {
                id: 4,
                parent_id: Some(2),
                path: "".to_string(),
                name: "AC".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag3".to_string(),
                }],
            },
            FolderResponse {
                id: 17,
                parent_id: Some(16),
                path: "".to_string(),
                name: "Z".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag1".to_string(),
                }],
            },
            FolderResponse {
                id: 2,
                parent_id: Some(1),
                path: "".to_string(),
                name: "AB".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![
                    TagApi {
                        id: None,
                        title: "tag2".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag1".to_string(),
                    },
                ],
            },
            FolderResponse {
                id: 15,
                parent_id: Some(14),
                path: "".to_string(),
                name: "X".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![
                    TagApi {
                        id: None,
                        title: "tag3".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag1".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag2".to_string(),
                    },
                ],
            },
            FolderResponse {
                id: 8,
                parent_id: Some(7),
                path: "".to_string(),
                name: "EC".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag3".to_string(),
                }],
            },
            FolderResponse {
                id: 3,
                parent_id: Some(1),
                path: "".to_string(),
                name: "ABB".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![
                    TagApi {
                        id: None,
                        title: "tag2".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag3".to_string(),
                    },
                ],
            },
            FolderResponse {
                id: 7,
                parent_id: Some(6),
                path: "".to_string(),
                name: "EB".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag2".to_string(),
                }],
            },
            FolderResponse {
                id: 1,
                parent_id: None,
                path: "".to_string(),
                name: "A".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag1".to_string(),
                }],
            },
            FolderResponse {
                id: 14,
                parent_id: None,
                path: "".to_string(),
                name: "XA".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag1".to_string(),
                }],
            },
        ]);

        let expected = HashSet::from([
            FolderResponse {
                id: 4,
                parent_id: Some(2),
                path: "".to_string(),
                name: "AC".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag3".to_string(),
                }],
            },
            FolderResponse {
                id: 8,
                parent_id: Some(7),
                path: "".to_string(),
                name: "EC".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag3".to_string(),
                }],
            },
            FolderResponse {
                id: 15,
                parent_id: Some(14),
                path: "".to_string(),
                name: "X".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![
                    TagApi {
                        id: None,
                        title: "tag1".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag2".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag3".to_string(),
                    },
                ],
            },
            FolderResponse {
                id: 3,
                parent_id: Some(1),
                path: "".to_string(),
                name: "ABB".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![
                    TagApi {
                        id: None,
                        title: "tag2".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag3".to_string(),
                    },
                ],
            },
        ])
        .into_iter()
        .map(|f| f.id)
        .collect::<HashSet<u32>>();

        let actual = reduce_folders_by_tag(
            &folders,
            &HashSet::from(["tag1".to_string(), "tag2".to_string(), "tag3".to_string()]),
        )
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect::<HashSet<u32>>();
        assert_eq!(expected, actual);
        cleanup();
    }

    #[test]
    fn reduce_folders_by_tag_keeps_first_folder_with_all_tags() {
        refresh_db();
        create_folder_db_entry("top", None); // 1
        create_folder_db_entry("middle", Some(1)); // 2
        create_folder_db_entry("bottom", Some(2)); // 3
        create_file_db_entry("top file", Some(1));
        create_file_db_entry("bottom file", Some(3));
        create_tag_folders("tag1", vec![1, 3]); // tag1 on top folder and bottom folder
        create_tag_folder("tag2", 3); // tag2 only on bottom folder
        let input_folders = HashSet::from([
            FolderResponse {
                id: 2,
                parent_id: Some(1),
                name: "middle".to_string(),
                path: "".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag2".to_string(),
                }],
            },
            FolderResponse {
                id: 3,
                parent_id: Some(2),
                name: "bottom".to_string(),
                path: "".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![TagApi {
                    id: None,
                    title: "tag1".to_string(),
                }],
            },
            FolderResponse {
                id: 1,
                parent_id: None,
                name: "top".to_string(),
                path: "".to_string(),
                folders: vec![],
                files: vec![],
                tags: vec![
                    TagApi {
                        id: None,
                        title: "tag1".to_string(),
                    },
                    TagApi {
                        id: None,
                        title: "tag2".to_string(),
                    },
                ],
            },
        ]);
        let expected: HashSet<u32> = HashSet::<u32>::from([1u32]);
        let actual: HashSet<u32> =
            reduce_folders_by_tag(&input_folders, &HashSet::from(["tag1".to_string()]))
                .unwrap()
                .into_iter()
                .map(|it| it.id)
                .collect();
        assert_eq!(expected, actual);
        cleanup();
    }
}
