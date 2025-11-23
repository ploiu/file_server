use std::{backtrace::Backtrace, collections::HashMap};

use rusqlite::Connection;

use crate::tags::TagTypes;

use super::models;

/// creates a new tag in the database. This does not check if the tag already exists,
/// so the caller must check that themselves
pub fn create_tag(title: &str, con: &Connection) -> Result<models::Tag, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/create_tag.sql"))?;
    let id = pst.insert(rusqlite::params![title])? as u32;
    Ok(models::Tag {
        id,
        title: title.to_string(),
    })
}

/// searches for a tag that case-insensitively matches that passed title.
///
/// if `None` is returned, that means there was no match
pub fn get_tag_by_title(
    title: &str,
    con: &Connection,
) -> Result<Option<models::Tag>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/get_by_title.sql"))?;
    match pst.query_row(rusqlite::params![title], tag_mapper) {
        Ok(tag) => Ok(Some(tag)),
        Err(e) => {
            // no tag found
            if e == rusqlite::Error::QueryReturnedNoRows {
                Ok(None)
            } else {
                log::error!(
                    "Failed to get tag by name, error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                Err(e)
            }
        }
    }
}

/// retrieves a tag from the database with the passed `id`
///
/// # Parameters
/// - `id`: the unique identifier of the tag to retrieve
/// - `con`: the database connection to use. Callers must handle closing the connection
///
/// # Returns
/// - `Ok(models::Tag)`: the tag with the specified ID if the tag exists
/// - `Err(rusqlite::Error)`: if there was an error during the database operation, including if no tag with the specified ID exists
pub fn get_tag(id: u32, con: &Connection) -> Result<models::Tag, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/get_by_id.sql"))?;
    pst.query_row(rusqlite::params![id], tag_mapper)
}

/// updates the past tag. Checking to make sure the tag exists needs to be done on the caller's end
pub fn update_tag(tag: models::Tag, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/update_tag.sql"))?;
    pst.execute(rusqlite::params![tag.title, tag.id])?;
    Ok(())
}

pub fn delete_tag(id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/delete_tag.sql"))?;
    pst.execute(rusqlite::params![id])?;
    Ok(())
}

// ================= file functions =================
/// the caller of this function will need to make sure the tag already exists and isn't already on the file
pub fn add_explicit_tag_to_file(
    file_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/add_tag_to_file.sql"))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

/// Adds an implicit tag to a file
///
/// This function will _only_ add a tag to a file if it doesn't already have that tag (explicit or implicit)
///
/// Parameters:
/// - `tag_id`: the id of the tag to add
/// - `file_id`: the id of the file to add the tag to
/// - `implicit_from_id`: the id of the folder that implicates the tag on the file
///
/// ## Returns:
/// will return a rusqlite error if a database interaction fails
pub fn add_implicit_tag_to_file(
    tag_id: u32,
    file_id: u32,
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/add_implicit_tag_to_file.sql"
    ))?;
    pst.execute(rusqlite::params![tag_id, file_id, implicit_from_id])?;
    Ok(())
}

/// Adds an implicit tag to multiple files
///
/// For each file, a tag is added _only_ if that file doesn't already have that tag (explicit or implicit)
///
/// ## Parameters:
/// - `tag_id`: the id of the tag to add
/// - `file_ids`: the ids of the files to add the tag to
/// - `implicit_from_id`: the id of the folder that implicates the tag on the files
/// - `con`: a reference to a database connection. The caller must manage closing the connection.
///
/// ## Returns:
/// will return a rusqlite error if a database interaction fails
///
/// ---
/// See also: [`add_implicit_tag_to_file`] for adding to a single file
pub fn add_implicit_tag_to_files(
    tag_id: u32,
    file_ids: &[u32],
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/add_implicit_tag_to_file.sql"
    ))?;
    for file_id in file_ids {
        pst.execute(rusqlite::params![tag_id, file_id, implicit_from_id])?;
    }
    Ok(())
}

/// Retrieves all tags on a file, explicit or implied
///
/// ## Parameters:
/// - `file_id` the id of the file to get tags for
/// - `con` a reference to a database connection. This must be closed by the parent
///
/// ## Returns:
/// - `Ok(Vec<models::TaggedItem>)`: a list of tags on the file
/// - `Err(rusqlite::Error)`: if there was an error during the database operation
///
/// If the file doesn't exist or has no tags, an empty vec is returned
pub fn get_all_tags_for_file(
    file_id: u32,
    con: &Connection,
) -> Result<Vec<models::TaggedItem>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/get_all_tags_for_file.sql"
    ))?;
    let rows = pst.query_map(rusqlite::params![file_id], tagged_item_mapper)?;
    let mut tags: Vec<models::TaggedItem> = Vec::new();
    for tag_res in rows {
        tags.push(tag_res?);
    }
    Ok(tags)
}

/// Retrieves all tags for a file with the passed id and tag type
///
/// ## Parameters
/// - `file_id`: the id of the file to get tags for
/// - `tag_type`: the type of tags to retrieve. If [`TagTypes::Explicit`] is passed, only tags explicitly passed on the file are returned.
///    If [`TagTypes::Implicit`] is passed, only implicated tags from parent folders are returned.
/// - `con`: a database connection to the database. Must be closed by the caller
///
/// See Also: [`get_all_tags_for_file`] to get all tags regardless of type
pub fn get_tags_for_file(
    file_id: u32,
    tag_type: TagTypes,
    con: &Connection,
) -> Result<Vec<models::TaggedItem>, rusqlite::Error> {
    let query = match tag_type {
        TagTypes::Explicit => include_str!("../assets/queries/tags/get_explicit_tags_for_file.sql"),
        TagTypes::Implicit => include_str!("../assets/queries/tags/get_implicit_tags_for_file.sql"),
    };
    let mut pst = con.prepare(query)?;
    let rows = pst.query_map(rusqlite::params![file_id], tagged_item_mapper)?;
    rows.collect::<Result<Vec<_>, _>>()
}

/// Retrieves all tags on all files passed, explicit or implied.
/// The returned value is a Map of file id => Vec<[`models::TaggedItem`]>. Files without _any_ tags will not have an entry in the map
///
/// ## Parameters:
/// - `file_ids` the ids to get tags for
/// - `con` a reference to a database connection. The caller must manage closing the connection.
///
/// ## Returns:
/// - `Ok(HashMap<u32, Vec<models::TaggedItem>>)` if the tags were successfully retrieved
/// - `Err(rusqlite::Error)` if there was a database error
///
/// ---
/// See also [`get_all_tags_for_file`]
///
pub fn get_all_tags_for_files(
    file_ids: Vec<u32>,
    con: &Connection,
) -> Result<HashMap<u32, Vec<models::TaggedItem>>, rusqlite::Error> {
    let in_clause: Vec<String> = file_ids.iter().map(|it| format!("'{it}'")).collect();
    let in_clause = in_clause.join(",");
    let formatted_query = format!(
        include_str!("../assets/queries/tags/get_all_tags_for_files.sql"),
        in_clause
    );
    let mut pst = con.prepare(formatted_query.as_str())?;
    let rows = pst.query_map([], tagged_item_mapper)?;
    let mut mapped: HashMap<u32, Vec<models::TaggedItem>> = HashMap::new();
    for tag in rows {
        let tag = tag?;
        let id = tag
            .file_id
            .expect("query should eliminate all non-file tags");
        mapped
            .entry(id)
            .and_modify(|tags| tags.push(tag.clone()))
            .or_insert_with(|| vec![tag]);
    }
    Ok(mapped)
}

/// removes the tag from the file if that file explicitly has that tag.
///
/// implicit tags are not removed with this function
pub fn remove_explicit_tag_from_file(
    file_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_explicit_tag_from_file.sql"
    ))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

/// Removes a single implied tag from all files that the passed `implicit_from_id` implicates the tag on
///
/// ## Parameters:
/// - `tag_id`: the tag to remove from those files
/// - `implicit_from_id`: the folder that was implicating the tag on the files
/// - `con`: a connection to the database. Must be closed by the caller
pub fn remove_implicit_tag_from_files(
    tag_id: u32,
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let query = include_str!("../assets/queries/tags/remove_implicit_tag_from_files.sql");
    let mut pst = con.prepare(&query)?;
    pst.execute(rusqlite::params![tag_id, implicit_from_id])?;
    Ok(())
}

/// Deletes an implicit tag from a file if it exists
pub fn remove_implicit_tag_from_file(
    tag_id: u32,
    file_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_implicit_tag_from_file.sql"
    ))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

// ================= folder functions =================
pub fn add_explicit_tag_to_folder(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/add_tag_to_folder.sql"))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    Ok(())
}

/// Adds an implicit tag to a folder (won't add if already exists)
pub fn add_implicit_tag_to_folder(
    tag_id: u32,
    folder_id: u32,
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/add_implicit_tag_to_folder.sql"
    ))?;
    pst.execute(rusqlite::params![tag_id, folder_id, implicit_from_id])?;
    Ok(())
}

/// Adds an implicit tag to multiple folders
///
/// For each folder, a tag is added _only_ if that folder doesn't already have that tag (explicit or implicit)
///
/// ## Parameters:
/// - `tag_id`: the id of the tag to add
/// - `folder_ids`: the ids of the folders to add the tag to
/// - `implicit_from_id`: the id of the folder that implicates the tag on the folders
/// - `con`: a reference to a database connection. The caller must manage closing the connection.
///
/// ## Returns:
/// will return a rusqlite error if a database interaction fails
///
/// ---
/// See also: [`add_implicit_tag_to_folder`] for adding to a single folder
pub fn add_implicit_tag_to_folders(
    tag_id: u32,
    folder_ids: &[u32],
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/add_implicit_tag_to_folder.sql"
    ))?;
    for folder_id in folder_ids {
        pst.execute(rusqlite::params![tag_id, folder_id, implicit_from_id])?;
    }
    Ok(())
}

/// Retrieves all tags on the folder with the passed id, explicit or implied.
/// If no folder is found, an empty Vec is returned.
///
/// ## Parameters:
/// - `folder_id` the id of the folder in the database to retrieve tags for
/// - `con` a reference to a database connection. The caller must manage closing the connection.
///
/// ## Returns:
/// - `Ok(Vec<models::TaggedItem>)` if the tags were successfully retrieved
/// - `Err(rusqlite::Error)` if there was a database error
pub fn get_all_tags_for_folder(
    folder_id: u32,
    con: &Connection,
) -> Result<Vec<models::TaggedItem>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/get_all_tags_for_folder.sql"
    ))?;
    let rows = pst.query_map(rusqlite::params![folder_id], tagged_item_mapper)?;
    rows.collect::<Result<Vec<models::TaggedItem>, rusqlite::Error>>()
}

/// Retrieves all tags for a folder with the passed id and tag type
///
/// ## Parameters
/// - `folder_id`: the id of the folder to get tags for
/// - `tag_type`: the type of tags to retrieve. If [`TagTypes::Explicit`] is passed, only tags explicitly passed on the folder are returned.
///    If [`TagTypes::Implicit`] is passed, only implicated tags from parent folders are returned.
/// - `con`: a database connection to the database. Must be closed by the caller
///
/// See Also: [`get_all_tags_for_folder`] to get all tags regardless of type
pub fn get_tags_for_folder(
    folder_id: u32,
    tag_type: TagTypes,
    con: &Connection,
) -> Result<Vec<models::TaggedItem>, rusqlite::Error> {
    let query = match tag_type {
        TagTypes::Explicit => {
            include_str!("../assets/queries/tags/get_explicit_tags_for_folder.sql")
        }
        TagTypes::Implicit => {
            include_str!("../assets/queries/tags/get_implicit_tags_for_folder.sql")
        }
    };
    let mut pst = con.prepare(query)?;
    let rows = pst.query_map(rusqlite::params![folder_id], tagged_item_mapper)?;
    rows.collect::<Result<Vec<_>, _>>()
}

pub fn remove_explicit_tag_from_folder(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_explicit_tag_from_folder.sql"
    ))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    Ok(())
}

/// Deletes an implicit tag from a folder if it exists
pub fn remove_implicit_tag_from_folder(
    tag_id: u32,
    folder_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_implicit_tag_from_folder.sql"
    ))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    Ok(())
}

/// Removes a single implicit tag from all folders that the passed `implicit_from_id` implicates the tag on
///
/// ## Parameters:
/// - `tag_id`: the tag to remove
/// - `implicit_from_id`: the folder that implicates the tag that should be removed
/// - `con`: a connection to the database. Must be closed by the caller
pub fn remove_implicit_tags_from_folders(
    tag_id: u32,
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let query = include_str!("../assets/queries/tags/remove_implicit_tags_from_folders.sql");
    let mut pst = con.prepare(&query)?;
    pst.execute(rusqlite::params![tag_id, implicit_from_id])?;
    Ok(())
}

// ================= both =================

/// for a given folder id, removes all implicit tags from descendants, so long as the tags being removed shouldn't be implied for the folder.
///
/// For example, if a folder has tags A, B, and C; all files and folders that have tags implicated by `implied_from_id` are removed _unless_ they are A, B, or C.
/// This can be used to clean up tags that used to be implied by the folder, but no longer are.
///
/// ## Parameters:
/// - `implied_from_id`: the id of the folder whose descendants need to have old implicated tags removed
/// - `con`: a connection to the database. Must be closed by the caller
pub fn remove_stale_implicit_tags_from_descendants(
    implied_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_stale_implicit_tags_from_descendants.sql"
    ))?;
    pst.execute([implied_from_id]).and(Ok(()))
}

// ================= misc =================
/// 1. id
/// 2. fileId
/// 3. folderId
/// 4. implicitFromId
/// 5. tagId
/// 6. title
fn tagged_item_mapper(row: &rusqlite::Row) -> Result<models::TaggedItem, rusqlite::Error> {
    let id: u32 = row.get(0)?;
    let file_id: Option<u32> = row.get(1)?;
    let folder_id: Option<u32> = row.get(2)?;
    let implicit_from_id: Option<u32> = row.get(3)?;
    let tag_id: u32 = row.get(4)?;
    let title: String = row.get(5)?;

    Ok(models::TaggedItem {
        id,
        file_id,
        folder_id,
        implicit_from_id,
        tag_id,
        title,
    })
}

/// maps a [`models::Tag`] from a database row
fn tag_mapper(row: &rusqlite::Row) -> Result<models::Tag, rusqlite::Error> {
    let id: u32 = row.get(0)?;
    let title: String = row.get(1)?;
    Ok(models::Tag { id, title })
}
