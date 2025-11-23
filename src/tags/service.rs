use std::backtrace::Backtrace;
use std::collections::HashSet;

use itertools::Itertools;

use super::{Tag, TagTypes};
use crate::model::error::file_errors::GetFileError;
use crate::model::error::tag_errors::{
    CreateTagError, DeleteTagError, GetTagError, TagRelationError, UpdateTagError,
};
use crate::model::response::{TagApi, TaggedItemApi};
use crate::repository::{folder_repository, open_connection};
use crate::service::{file_service, folder_service};
use crate::tags::repository;
use crate::tags::repository as tag_repository;

/// will create a tag, or return the already-existing tag if one with the same name exists
/// returns the created/existing tag
pub fn create_tag(name: String) -> Result<TagApi, CreateTagError> {
    let con = open_connection();
    let existing_tag: Option<Tag> = match tag_repository::get_tag_by_title(&name, &con) {
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
    let tag: Tag = if let Some(t) = existing_tag {
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
    let tag: Tag = match tag_repository::get_tag(id, &con) {
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
    let db_tag = Tag {
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
/// Only explict tags can be managed this way.
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
pub fn update_file_tags(file_id: u32, tags: Vec<TaggedItemApi>) -> Result<(), TagRelationError> {
    // make sure the file exists
    if Err(GetFileError::NotFound) == file_service::get_file_metadata(file_id) {
        log::error!(
            "Cannot update tag for file {file_id}, because that file does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let con = open_connection();
    // instead of removing all the tags and then adding them back, we can use a HashSet or 2 to enforce a unique list in-memory without as much IO
    let existing_tags: HashSet<TaggedItemApi> = HashSet::from_iter(get_tags_on_file(file_id)?);
    let tags = HashSet::from_iter(tags);
    // we need to find 2 things: 1) tags to add 2) tags to remove
    let tags_to_remove = existing_tags.difference(&tags);
    let tags_to_add = tags.difference(&existing_tags);
    for tag in tags_to_remove {
        // tags from the db will always have a non-None tag id
        if let Err(e) =
            tag_repository::remove_explicit_tag_from_file(file_id, tag.tag_id.unwrap(), &con)
        {
            log::error!(
                "Failed to remove tags from file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    }
    for tag in tags_to_add {
        let created = match create_tag(tag.title.clone()) {
            Ok(t) => t,
            Err(e) => {
                con.close().unwrap();
                log::error!(
                    "Failed to create tag! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };
        if let Err(e) = tag_repository::add_explicit_tag_to_file(file_id, created.id.unwrap(), &con)
        {
            con.close().unwrap();
            log::error!(
                "Failed to add tag to file: {e:?}\n{}",
                Backtrace::force_capture(),
            );
            return Err(TagRelationError::DbError);
        }
    }
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
pub fn update_folder_tags(
    folder_id: u32,
    tags: Vec<TaggedItemApi>,
) -> Result<(), TagRelationError> {
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
        if let Err(e) =
            tag_repository::remove_explicit_tag_from_folder(folder_id, tag.tag_id.unwrap(), &con)
        {
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
    let existing_tags: Vec<&TaggedItemApi> = tags.iter().filter(|t| t.tag_id.is_some()).collect();
    for tag in existing_tags {
        let tag_id = tag.tag_id.unwrap();
        // Skip if we've already added this tag
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_explicit_tag_to_folder(folder_id, tag_id, &con) {
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
    let new_tags: Vec<&TaggedItemApi> = tags.iter().filter(|t| t.tag_id.is_none()).collect();
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
        if let Err(e) = tag_repository::add_explicit_tag_to_folder(folder_id, tag_id, &con) {
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

    // Propagate tag changes to all descendants
    pass_tags_to_children(folder_id)?;

    Ok(())
}

/// retrieves all the tags on the file with the passed id
pub fn get_tags_on_file(file_id: u32) -> Result<Vec<TaggedItemApi>, TagRelationError> {
    // make sure the file exists
    if !file_service::check_file_exists(file_id) {
        log::error!(
            "Cannot get tags on file with id {file_id}, because that file does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let con: rusqlite::Connection = open_connection();
    let file_tags = match tag_repository::get_all_tags_for_file(file_id, &con) {
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
    Ok(file_tags.into_iter().map_into().collect())
}

/// retrieves all the tags on the folder with the passed id.
/// This will always be empty if requesting with the root folder id (0 or None)
pub fn get_tags_on_folder(folder_id: u32) -> Result<Vec<TaggedItemApi>, TagRelationError> {
    // make sure the folder exists
    if !folder_service::folder_exists(Some(folder_id)) {
        log::error!(
            "Cannot get tags on folder with id {folder_id}, because that folder does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let con: rusqlite::Connection = open_connection();
    let db_tags = match tag_repository::get_all_tags_for_folder(folder_id, &con) {
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
    Ok(db_tags.into_iter().map(TaggedItemApi::from).collect())
}

/// Propagates tag changes from a folder to all its descendant files and folders.
///
/// This function ensures that:
/// - All explicit tags on the folder are implied to descendants (if not already present)
/// - All removed explicit tags have their implications removed from descendants
/// - Explicit tags on descendants are never overridden
///
/// ## Parameters
/// - `folder_id`: the id of the folder whose tags should be propagated to descendants
///
/// ## Returns
/// - `Ok(())` if tags were successfully propagated
/// - `Err(TagRelationError)` if there was a database error or the folder doesn't exist
pub fn pass_tags_to_children(folder_id: u32) -> Result<(), TagRelationError> {
    // Verify folder exists
    if !folder_service::folder_exists(Some(folder_id)) {
        log::error!(
            "Cannot pass tags to children of folder {folder_id} because it does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FolderNotFound);
    }

    let con = open_connection();

    // Get all explicit tags on this folder
    let explicit_tags =
        match tag_repository::get_tags_for_folder(folder_id, TagTypes::Explicit, &con) {
            Ok(tags) => tags,
            Err(e) => {
                log::error!(
                    "Failed to retrieve tags on folder {folder_id}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        };

    // Get all descendant folders, which doubles as a way to get all descendant files later
    let mut all_folder_ids = match folder_repository::get_all_child_folder_ids(
        &vec![folder_id],
        &con,
    ) {
        Ok(folders) => folders,
        Err(e) => {
            log::error!(
                "Failed to retrieve descendant folders for folder {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };
    // need to add the original folder id so that it's truly all folder ids involved
    all_folder_ids.push(folder_id);
    let descendant_files: Vec<u32> = match folder_repository::get_child_files(all_folder_ids, &con)
    {
        Ok(files) => files.into_iter().map(|f| f.id.unwrap()).collect(),
        Err(e) => {
            log::error!(
                "Failed to retrieve descendant files for folder {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };

    // now that we have all descendant folders and files, we need to remove all implicated tags that shouldn't be there
    if let Err(e) = repository::remove_stale_implicit_tags_from_descendants(folder_id, &con) {
        con.close().unwrap();
        log::error!(
            "Failed to remove implicit tags from descendants of folder {folder_id}! Error is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::DbError);
    }
    // stale implied tags are removed, affected files and folders now need to be updated to re-inherit from folders that have that tag.
    // This is because a higher parent could have received that tag after `folder_id` got it. It shouldn't be that a child folder having its tags changed should cause this,
    // because adding a tag to a folder should be blocked if a parent has that tag.
    let all_ancestor_ids = match folder_repository::get_ancestor_folders_with_id(folder_id, &con) {
        Ok(ids) => ids,
        Err(e) => {
            con.close().unwrap();
            log::error!(
                "Failed to retrieve ancestor folders for folder {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(TagRelationError::DbError);
        }
    };
    // TODO get all tags for ancestor IDs, get the explicit ones, and make all children inherit them (use insert or ignore)
    con.close().unwrap();
    Ok(())
}
