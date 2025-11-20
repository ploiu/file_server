use std::backtrace::Backtrace;
use std::collections::HashSet;

use itertools::Itertools;
use rusqlite::Connection;

use crate::model::error::file_errors::GetFileError;
use crate::model::error::tag_errors::{
    CreateTagError, DeleteTagError, GetTagError, TagRelationError, UpdateTagError,
};
use crate::model::repository::{self};
use crate::model::response::{TagApi, TaggedItemApi};
use crate::repository::{folder_repository, open_connection};
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
        if let Err(e) = tag_repository::remove_tag_from_folder(folder_id, tag.tag_id.unwrap(), &con)
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
    let folder_tags = match tag_repository::get_tags_on_folder(folder_id, &con) {
        Ok(tags) => tags
            .into_iter()
            .filter(|t| t.implicit_from_id.is_none())
            .collect::<Vec<_>>(),
        Err(e) => {
            log::error!(
                "Failed to retrieve tags on folder {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };

    // Get all descendant folders and files
    let descendant_folders = match folder_repository::get_all_child_folder_ids(&vec![folder_id], &con) {
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

    // Get files from the folder and all its descendants
    let mut all_folder_ids = vec![folder_id];
    all_folder_ids.extend(&descendant_folders);
    let descendant_files: Vec<u32> = match folder_repository::get_child_files(all_folder_ids, &con) {
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

    // Get all tag IDs that this folder has explicitly
    let folder_tag_ids: HashSet<u32> = folder_tags.iter().map(|t| t.tag_id).collect();

    // Remove all implicit tags from descendants that the folder doesn't have
    // This handles the case where a tag was removed from the folder
    if let Err(e) = remove_orphaned_implications(
        folder_id,
        &descendant_folders,
        &descendant_files,
        &folder_tag_ids,
        &con,
    ) {
        con.close().unwrap();
        return Err(e);
    }

    // Add implications for all tags the folder has
    for tag in folder_tags {
        if let Err(e) =
            add_tag_to_descendants(tag.tag_id, folder_id, &descendant_folders, &descendant_files, &con)
        {
            con.close().unwrap();
            return Err(e);
        }
    }

    con.close().unwrap();
    Ok(())
}

/// Removes implicit tags from descendants that are inherited from this folder but the folder no longer has
fn remove_orphaned_implications(
    folder_id: u32,
    descendant_folders: &[u32],
    descendant_files: &[u32],
    current_tag_ids: &HashSet<u32>,
    con: &Connection,
) -> Result<(), TagRelationError> {
    // Get all unique tag IDs that are currently implied from this folder to any descendant
    let mut implied_tags: HashSet<u32> = HashSet::new();

    // Check folders
    for folder in descendant_folders {
        let tags = match tag_repository::get_tags_on_folder(*folder, con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to retrieve tags on folder {folder}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };
        for tag in tags {
            if tag.implicit_from_id == Some(folder_id) {
                implied_tags.insert(tag.tag_id);
            }
        }
    }

    // Check files
    for file in descendant_files {
        let tags = match tag_repository::get_tags_on_file(*file, con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to retrieve tags on file {file}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };
        for tag in tags {
            if tag.implicit_from_id == Some(folder_id) {
                implied_tags.insert(tag.tag_id);
            }
        }
    }

    // Remove implications for tags that are no longer on the folder
    for tag_id in implied_tags {
        if !current_tag_ids.contains(&tag_id) {
            // Remove from folders
            if let Err(e) = tag_repository::remove_implicit_tags_from_folders(
                descendant_folders,
                tag_id,
                folder_id,
                con,
            ) {
                log::error!(
                    "Failed to remove implicit tag {tag_id} from descendant folders! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }

            // Remove from files
            if let Err(e) = tag_repository::remove_implicit_tags_from_files(
                descendant_files,
                tag_id,
                folder_id,
                con,
            ) {
                log::error!(
                    "Failed to remove implicit tag {tag_id} from descendant files! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }

            // After removing the tag, check if any descendant needs to re-inherit from a higher ancestor
            if let Err(e) = re_inherit_from_ancestors(
                folder_id,
                tag_id,
                descendant_folders,
                descendant_files,
                con,
            ) {
                return Err(e);
            }
        }
    }

    Ok(())
}

/// After removing an implicit tag, check if descendants need to inherit it from a higher ancestor
fn re_inherit_from_ancestors(
    _removed_from_folder_id: u32,
    tag_id: u32,
    descendant_folders: &[u32],
    descendant_files: &[u32],
    con: &Connection,
) -> Result<(), TagRelationError> {
    // For each descendant folder, walk up the parent chain to find if any ancestor has this tag
    for folder_id in descendant_folders {
        if let Some(new_implicit_from) = find_ancestor_with_tag(*folder_id, tag_id, con)? {
            // Only re-inherit if the folder doesn't have the tag explicitly
            let tags = match tag_repository::get_tags_on_folder(*folder_id, con) {
                Ok(t) => t,
                Err(e) => {
                    log::error!(
                        "Failed to get tags for folder {folder_id}! Error is {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    return Err(TagRelationError::DbError);
                }
            };
            let has_explicit = tags
                .iter()
                .any(|t| t.tag_id == tag_id && t.implicit_from_id.is_none());
            if !has_explicit {
                if let Err(e) =
                    tag_repository::add_implicit_tag_to_folder(tag_id, *folder_id, new_implicit_from, con)
                {
                    log::error!(
                        "Failed to re-inherit tag {tag_id} to folder {folder_id}! Error is {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    return Err(TagRelationError::DbError);
                }
            }
        }
    }

    // For each descendant file, walk up the parent chain to find if any ancestor has this tag
    for file_id in descendant_files {
        if let Some(new_implicit_from) = find_ancestor_with_tag_for_file(*file_id, tag_id, con)? {
            // Only re-inherit if the file doesn't have the tag explicitly
            let tags = match tag_repository::get_tags_on_file(*file_id, con) {
                Ok(t) => t,
                Err(e) => {
                    log::error!(
                        "Failed to get tags for file {file_id}! Error is {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    return Err(TagRelationError::DbError);
                }
            };
            let has_explicit = tags
                .iter()
                .any(|t| t.tag_id == tag_id && t.implicit_from_id.is_none());
            if !has_explicit {
                if let Err(e) =
                    tag_repository::add_implicit_tag_to_file(tag_id, *file_id, new_implicit_from, con)
                {
                    log::error!(
                        "Failed to re-inherit tag {tag_id} to file {file_id}! Error is {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    return Err(TagRelationError::DbError);
                }
            }
        }
    }

    Ok(())
}

/// Finds the nearest ancestor folder that has the specified tag explicitly
fn find_ancestor_with_tag(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<Option<u32>, TagRelationError> {
    // Get the folder to find its parent
    let folder = match folder_repository::get_by_id(Some(folder_id), con) {
        Ok(f) => f,
        Err(e) => {
            log::error!(
                "Failed to get folder {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(TagRelationError::DbError);
        }
    };

    let mut current_parent = folder.parent_id;

    // Walk up the parent chain
    while let Some(parent_id) = current_parent {
        // Check if this parent has the tag explicitly
        let tags = match tag_repository::get_tags_on_folder(parent_id, con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to get tags for folder {parent_id}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };

        if tags
            .iter()
            .any(|t| t.tag_id == tag_id && t.implicit_from_id.is_none())
        {
            return Ok(Some(parent_id));
        }

        // Move to the next parent
        let parent = match folder_repository::get_by_id(Some(parent_id), con) {
            Ok(f) => f,
            Err(e) => {
                log::error!(
                    "Failed to get folder {parent_id}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };
        current_parent = parent.parent_id;
    }

    Ok(None)
}

/// Finds the nearest ancestor folder of a file that has the specified tag explicitly
fn find_ancestor_with_tag_for_file(
    file_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<Option<u32>, TagRelationError> {
    // Get the file's parent folder
    let file_record = match file_service::get_file_metadata(file_id) {
        Ok(f) => f,
        Err(e) => {
            log::error!(
                "Failed to get file {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(TagRelationError::DbError);
        }
    };

    let mut current_parent = file_record.folder_id;

    // Walk up the folder parent chain
    while let Some(parent_id) = current_parent {
        // Check if this folder has the tag explicitly
        let tags = match tag_repository::get_tags_on_folder(parent_id, con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to get tags for folder {parent_id}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };

        if tags
            .iter()
            .any(|t| t.tag_id == tag_id && t.implicit_from_id.is_none())
        {
            return Ok(Some(parent_id));
        }

        // Move to the next parent
        let parent = match folder_repository::get_by_id(Some(parent_id), con) {
            Ok(f) => f,
            Err(e) => {
                log::error!(
                    "Failed to get folder {parent_id}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };
        current_parent = parent.parent_id;
    }

    Ok(None)
}

/// Adds a tag to all descendants that don't already have it explicitly or from a closer ancestor
fn add_tag_to_descendants(
    tag_id: u32,
    folder_id: u32,
    descendant_folders: &[u32],
    descendant_files: &[u32],
    con: &Connection,
) -> Result<(), TagRelationError> {
    // For each descendant folder, check if it should have this implicit tag
    for descendant_folder_id in descendant_folders {
        let tags = match tag_repository::get_tags_on_folder(*descendant_folder_id, con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to get tags for folder {descendant_folder_id}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };

        // Check if folder has this tag explicitly - if so, don't override
        let has_explicit = tags
            .iter()
            .any(|t| t.tag_id == tag_id && t.implicit_from_id.is_none());
        
        if has_explicit {
            continue;
        }

        // Check if folder has this tag implicitly from a closer ancestor (descendant of current folder)
        // If the implicit_from_id is in descendant_folders, it means it's closer than folder_id
        if let Some(existing_implicit) = tags.iter().find(|t| t.tag_id == tag_id && t.implicit_from_id.is_some()) {
            if let Some(implicit_from) = existing_implicit.implicit_from_id {
                // If the folder already inherits from a descendant of current folder, keep it
                if descendant_folders.contains(&implicit_from) {
                    continue;
                }
            }
        }

        // Add or update the implicit tag
        if let Err(e) = tag_repository::upsert_implicit_tag_to_folder(tag_id, *descendant_folder_id, folder_id, con) {
            log::error!(
                "Failed to upsert implicit tag {tag_id} to folder {descendant_folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(TagRelationError::DbError);
        }
    }

    // For each descendant file, check if it should have this implicit tag
    for descendant_file_id in descendant_files {
        let tags = match tag_repository::get_tags_on_file(*descendant_file_id, con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to get tags for file {descendant_file_id}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return Err(TagRelationError::DbError);
            }
        };

        // Check if file has this tag explicitly - if so, don't override
        let has_explicit = tags
            .iter()
            .any(|t| t.tag_id == tag_id && t.implicit_from_id.is_none());
        
        if has_explicit {
            continue;
        }

        // Check if file has this tag implicitly from a closer ancestor (descendant folder of current folder)
        // Get the file's parent folder and check if it's a descendant of folder_id
        if let Some(existing_implicit) = tags.iter().find(|t| t.tag_id == tag_id && t.implicit_from_id.is_some()) {
            if let Some(implicit_from) = existing_implicit.implicit_from_id {
                // If the file already inherits from a descendant of current folder, keep it
                // This includes the direct parent and any ancestor folders that are descendants of folder_id
                if descendant_folders.contains(&implicit_from) {
                    continue;
                }
            }
        }

        // Add or update the implicit tag
        if let Err(e) = tag_repository::upsert_implicit_tag_to_file(tag_id, *descendant_file_id, folder_id, con) {
            log::error!(
                "Failed to upsert implicit tag {tag_id} to file {descendant_file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(TagRelationError::DbError);
        }
    }

    Ok(())
}
