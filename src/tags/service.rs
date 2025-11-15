use std::backtrace::Backtrace;
use std::collections::HashSet;

use crate::model::error::file_errors::GetFileError;
use crate::model::error::tag_errors::{
    CreateTagError, DeleteTagError, GetTagError, TagRelationError, UpdateTagError,
};
use crate::model::repository;
use crate::model::response::TagApi;
use crate::repository::open_connection;
use crate::service::{file_service, folder_service};
use crate::tags::repository as tag_repository;

/// will create a tag, or return the already-existing tag if one with the same name exists
/// returns the created/existing tag
pub fn create_tag(name: String) -> Result<TagApi, CreateTagError> {
    let con = open_connection();
    let existing_tag: Option<repository::Tag> = match tag_repository::get_tag_by_title(&name, &con)
    {
        Ok(tags) => tags,
        Err(e) => {
            log::error!(
                "Failed to check if any tags with the name {name} already exist! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(CreateTagError::DbError);
        }
    };
    let tag: repository::Tag = if let Some(t) = existing_tag {
        t
    } else {
        match tag_repository::create_tag(&name, &con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to create a new tag with the name {name}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(CreateTagError::DbError);
            }
        }
    };

    con.close().unwrap();
    Ok(TagApi::from(tag))
}

/// will return the tag with the passed id
pub fn get_tag(id: u32) -> Result<TagApi, GetTagError> {
    let con = open_connection();
    let tag: repository::Tag = match tag_repository::get_tag(id, &con) {
        Ok(t) => t,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            log::error!(
                "No tag with id {id} exists!\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(GetTagError::TagNotFound);
        }
        Err(e) => {
            log::error!(
                "Could not retrieve tag with id {id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(GetTagError::DbError);
        }
    };
    con.close().unwrap();
    Ok(TagApi::from(tag))
}

/// updates the tag with the passed id to the passed name.
/// Will fail if a tag already exists with that name
pub fn update_tag(request: TagApi) -> Result<TagApi, UpdateTagError> {
    let con: rusqlite::Connection = open_connection();
    // make sure the tag exists first TODO cleanup - use if let Err pattern since Ok branch is empty
    match tag_repository::get_tag(request.id.unwrap(), &con) {
        Ok(_) => { /* no op */ }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            log::error!(
                "Could not update tag with id {:?}, because it does not exist!\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::TagNotFound);
        }
        Err(e) => {
            log::error!(
                "Could not update tag with id {:?}! Error is {e}\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    let new_title = request.title;
    // now make sure the database doesn't already have a tag with the new name TODO maybe see if can clean up, 2 empty branches is a smell
    match tag_repository::get_tag_by_title(&new_title, &con) {
        Ok(Some(_)) => {
            log::error!(
                "Could not update tag with id {:?} to name {new_title}, because a tag with that name already exists!\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::NewNameAlreadyExists);
        }
        Ok(None) => {}
        Err(rusqlite::Error::QueryReturnedNoRows) => { /* this is the good route - no op */ }
        Err(e) => {
            log::error!(
                "Could not search tags by name with value {new_title}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    // no match, and tag already exists so we're good to go
    let db_tag = repository::Tag {
        id: request.id.unwrap(),
        title: new_title.clone(),
    };
    match tag_repository::update_tag(db_tag, &con) {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Could not update tag with id {:?}! Error is {e}\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    con.close().unwrap();
    Ok(TagApi {
        id: request.id,
        title: new_title,
    })
}

/// deletes the tag with the passed id. Does nothing if that tag doesn't exist
pub fn delete_tag(id: u32) -> Result<(), DeleteTagError> {
    let con: rusqlite::Connection = open_connection();
    // TODO change to if let Err pattern, Ok branch is empty
    match tag_repository::delete_tag(id, &con) {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Could not delete tag with id {id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(DeleteTagError::DbError);
        }
    };
    con.close().unwrap();
    Ok(())
}

/// Updates the tags on a file by replacing all existing tags with the provided list.
///
/// This function will:
/// 1. Remove all existing tags from the file
/// 2. Add tags that already exist in the database (those with an `id`)
/// 3. Create and add new tags (those without an `id`)
///
/// Duplicate tags in the input list will be automatically deduplicated to prevent
/// database constraint violations.
///
/// # Parameters
/// - `file_id`: The ID of the file to update tags for
/// - `tags`: A vector of tags to set on the file. Tags with an `id` will be linked directly,
///   tags without an `id` will be created first (or retrieved if they already exist by name)
///
/// # Returns
/// - `Ok(())` if the tags were successfully updated
/// - `Err(TagRelationError::FileNotFound)` if the file does not exist
/// - `Err(TagRelationError::DbError)` if there was a database error
pub fn update_file_tags(file_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    // make sure the file exists
    if Err(GetFileError::NotFound) == file_service::get_file_metadata(file_id) {
        log::error!(
            "Cannot update tag for file {file_id}, because that file does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let existing_tags = get_tags_on_file(file_id)?;
    let con: rusqlite::Connection = open_connection();
    // Remove all existing tags from the file
    for tag in existing_tags.iter() {
        // tags from the db will always have a non-None tag id
        if let Err(e) = tag_repository::remove_tag_from_file(file_id, tag.id.unwrap(), &con) {
            log::error!(
                "Failed to remove tag from file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    }

    // Track which tag IDs have been added to avoid duplicates
    let mut added_tag_ids: HashSet<u32> = HashSet::new();

    // First, add all existing tags (those with an id)
    let existing_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_some()).collect();
    for tag in existing_tags {
        let tag_id = tag.id.unwrap();
        // Skip if we've already added this tag
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_file(file_id, tag_id, &con) {
            log::error!(
                "Failed to add tag to file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    // Then, create and add new tags (those without an id)
    let new_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_none()).collect();
    for tag in new_tags {
        let created_tag = match create_tag(tag.title.clone()) {
            Ok(t) => t,
            Err(_) => {
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        };
        let tag_id = created_tag.id.unwrap();
        // Skip if we've already added this tag (prevents duplicates)
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_file(file_id, tag_id, &con) {
            log::error!(
                "Failed to add tag to file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    con.close().unwrap();
    Ok(())
}

/// Updates the tags on a folder by replacing all existing tags with the provided list.
///
/// This function will:
/// 1. Remove all existing tags from the folder
/// 2. Add tags that already exist in the database (those with an `id`)
/// 3. Create and add new tags (those without an `id`)
///
/// Duplicate tags in the input list will be automatically deduplicated to prevent
/// database constraint violations.
///
/// # Parameters
/// - `folder_id`: The ID of the folder to update tags for
/// - `tags`: A vector of tags to set on the folder. Tags with an `id` will be linked directly,
///   tags without an `id` will be created first (or retrieved if they already exist by name)
///
/// # Returns
/// - `Ok(())` if the tags were successfully updated
/// - `Err(TagRelationError::FolderNotFound)` if the folder does not exist
/// - `Err(TagRelationError::DbError)` if there was a database error
pub fn update_folder_tags(folder_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    // make sure the file exists
    if !folder_service::folder_exists(Some(folder_id)) {
        log::error!("Cannot update tags for a folder that does not exist (id {folder_id}!");
        return Err(TagRelationError::FolderNotFound);
    }
    let existing_tags = get_tags_on_folder(folder_id)?;
    let con: rusqlite::Connection = open_connection();
    // Remove all existing tags from the folder
    for tag in existing_tags.iter() {
        // tags from the db will always have a non-None tag id
        if let Err(e) = tag_repository::remove_tag_from_folder(folder_id, tag.id.unwrap(), &con) {
            log::error!(
                "Failed to remove tags from folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    }

    // Track which tag IDs have been added to avoid duplicates
    let mut added_tag_ids: HashSet<u32> = HashSet::new();

    // First, add all existing tags (those with an id)
    let existing_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_some()).collect();
    for tag in existing_tags {
        let tag_id = tag.id.unwrap();
        // Skip if we've already added this tag
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_folder(folder_id, tag_id, &con) {
            log::error!(
                "Failed to add tags to folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    // Then, create and add new tags (those without an id)
    let new_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_none()).collect();
    for tag in new_tags {
        let created_tag = match create_tag(tag.title.clone()) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to create tag! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        };
        let tag_id = created_tag.id.unwrap();
        // Skip if we've already added this tag (prevents duplicates)
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_folder(folder_id, tag_id, &con) {
            log::error!(
                "Failed to add tags to folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    con.close().unwrap();
    
    // Pass tags to all descendants
    pass_tags_to_children(folder_id)?;
    
    Ok(())
}

/// retrieves all the tags on the file with the passed id
pub fn get_tags_on_file(file_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    // make sure the file exists
    if !file_service::check_file_exists(file_id) {
        log::error!(
            "Cannot get tags on file with id {file_id}, because that file does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let con: rusqlite::Connection = open_connection();
    let file_tags = match tag_repository::get_tags_on_file(file_id, &con) {
        Ok(tags) => tags,
        Err(e) => {
            log::error!(
                "Failed to retrieve tags on file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };
    con.close().unwrap();
    let api_tags: Vec<TagApi> = file_tags.into_iter().map(TagApi::from).collect();
    Ok(api_tags)
}

/// retrieves all the tags on the folder with the passed id.
/// This will always be empty if requesting with the root folder id (0 or None)
pub fn get_tags_on_folder(folder_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    // make sure the folder exists
    if !folder_service::folder_exists(Some(folder_id)) {
        log::error!(
            "Cannot get tags on folder with id {folder_id}, because that folder does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let con: rusqlite::Connection = open_connection();
    let db_tags = match tag_repository::get_tags_on_folder(folder_id, &con) {
        Ok(tags) => tags,
        Err(e) => {
            log::error!(
                "Failed to retrieve tags on folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };
    con.close().unwrap();
    let api_tags: Vec<TagApi> = db_tags.into_iter().map(TagApi::from).collect();
    Ok(api_tags)
}

/// Pass tags to all descendant files and folders.
/// 
/// This function ensures that all descendants (files and folders) of the given folder
/// inherit exactly the tags that are directly on the folder, and no tags that aren't.
/// 
/// # Algorithm
/// 1. Retrieve all tags directly on the folder (not inherited)
/// 2. Retrieve all descendant folders recursively
/// 3. Retrieve all descendant files (in the folder and all its descendants)
/// 4. For each descendant (folder or file):
///    - Remove any inherited tags that are NOT on the parent folder
///    - Add inherited tags for any tags on the parent folder that the descendant doesn't have
/// 
/// # Parameters
/// - `folder_id`: The ID of the folder whose tags should be passed to children
/// 
/// # Returns
/// - `Ok(())` if tags were successfully passed to all descendants
/// - `Err(TagRelationError)` if there was an error
pub fn pass_tags_to_children(folder_id: u32) -> Result<(), TagRelationError> {
    // Make sure the folder exists
    if !folder_service::folder_exists(Some(folder_id)) {
        log::error!("Cannot pass tags to children for folder that does not exist (id {folder_id})!");
        return Err(TagRelationError::FolderNotFound);
    }

    let con = open_connection();
    
    // Get folder tag IDs (only direct tags, not inherited)
    let folder_tag_ids: HashSet<u32> = {
        let query = "SELECT tagId FROM TaggedItems WHERE folderId = ? AND inheritedFromId IS NULL";
        let mut pst = con.prepare(query).map_err(|e| {
            log::error!("Failed to prepare query: {e:?}\n{}", Backtrace::force_capture());
            TagRelationError::DbError
        })?;
        
        let rows = pst.query_map(rusqlite::params![folder_id], |row| row.get::<_, u32>(0)).map_err(|e| {
            log::error!("Failed to query folder tags: {e:?}\n{}", Backtrace::force_capture());
            TagRelationError::DbError
        })?;
        
        rows.filter_map(Result::ok).collect()
    };
    
    // Get all descendant folder IDs
    let descendant_folder_ids = tag_repository::get_descendant_folder_ids(folder_id, &con).map_err(|e| {
        log::error!(
            "Failed to retrieve descendant folders for folder {folder_id}! Error is {e:?}\n{}",
            Backtrace::force_capture()
        );
        TagRelationError::DbError
    })?;
    
    // Get all descendant file IDs
    let descendant_file_ids = tag_repository::get_descendant_file_ids(folder_id, &con).map_err(|e| {
        log::error!(
            "Failed to retrieve descendant files for folder {folder_id}! Error is {e:?}\n{}",
            Backtrace::force_capture()
        );
        TagRelationError::DbError
    })?;
    
    // Process descendant folders
    for desc_folder_id in descendant_folder_ids {
        // Remove inherited tags that are NOT on the parent folder
        let remove_query = "DELETE FROM TaggedItems WHERE folderId = ? AND inheritedFromId = ? AND tagId NOT IN (SELECT tagId FROM TaggedItems WHERE folderId = ? AND inheritedFromId IS NULL)";
        con.execute(remove_query, rusqlite::params![desc_folder_id, folder_id, folder_id]).map_err(|e| {
            log::error!("Failed to remove obsolete inherited tags from folder {desc_folder_id}: {e:?}\n{}", Backtrace::force_capture());
            TagRelationError::DbError
        })?;
        
        // Add inherited tags for tags on the parent folder that the descendant doesn't have
        for tag_id in &folder_tag_ids {
            let insert_query = "INSERT OR IGNORE INTO TaggedItems (folderId, tagId, inheritedFromId) VALUES (?, ?, ?)";
            con.execute(insert_query, rusqlite::params![desc_folder_id, tag_id, folder_id]).map_err(|e| {
                log::error!("Failed to add inherited tag {tag_id} to folder {desc_folder_id}: {e:?}\n{}", Backtrace::force_capture());
                TagRelationError::DbError
            })?;
        }
    }
    
    // Process descendant files  
    for file_id in descendant_file_ids {
        // Remove inherited tags that are NOT on the parent folder
        let remove_query = "DELETE FROM TaggedItems WHERE fileId = ? AND inheritedFromId = ? AND tagId NOT IN (SELECT tagId FROM TaggedItems WHERE folderId = ? AND inheritedFromId IS NULL)";
        con.execute(remove_query, rusqlite::params![file_id, folder_id, folder_id]).map_err(|e| {
            log::error!("Failed to remove obsolete inherited tags from file {file_id}: {e:?}\n{}", Backtrace::force_capture());
            TagRelationError::DbError
        })?;
        
        // Add inherited tags for tags on the parent folder that the file doesn't have
        for tag_id in &folder_tag_ids {
            let insert_query = "INSERT OR IGNORE INTO TaggedItems (fileId, tagId, inheritedFromId) VALUES (?, ?, ?)";
            con.execute(insert_query, rusqlite::params![file_id, tag_id, folder_id]).map_err(|e| {
                log::error!("Failed to add inherited tag {tag_id} to file {file_id}: {e:?}\n{}", Backtrace::force_capture());
                TagRelationError::DbError
            })?;
        }
    }
    
    con.close().unwrap();
    Ok(())
}
