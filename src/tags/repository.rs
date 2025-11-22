use std::{backtrace::Backtrace, collections::HashMap};

use itertools::Itertools;
use rusqlite::Connection;

use crate::{
    model::{repository, response::TaggedItemApi},
    tags::TagTypes,
};

/// creates a new tag in the database. This does not check if the tag already exists,
/// so the caller must check that themselves
pub fn create_tag(title: &str, con: &Connection) -> Result<repository::Tag, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/create_tag.sql"))?;
    let id = pst.insert(rusqlite::params![title])? as u32;
    Ok(repository::Tag {
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
) -> Result<Option<repository::Tag>, rusqlite::Error> {
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
/// - `Ok(repository::Tag)`: the tag with the specified ID if the tag exists
/// - `Err(rusqlite::Error)`: if there was an error during the database operation, including if no tag with the specified ID exists
pub fn get_tag(id: u32, con: &Connection) -> Result<repository::Tag, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/get_by_id.sql"))?;
    pst.query_row(rusqlite::params![id], tag_mapper)
}

/// updates the past tag. Checking to make sure the tag exists needs to be done on the caller's end
pub fn update_tag(tag: repository::Tag, con: &Connection) -> Result<(), rusqlite::Error> {
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

/// Retrieves all tags on a file, explicit or implied
///
/// ## Parameters:
/// - `file_id` the id of the file to get tags for
/// - `con` a reference to a database connection. This must be closed by the parent
///
/// ## Returns:
/// - `Ok(Vec<repository::TaggedItem>)`: a list of tags on the file
/// - `Err(rusqlite::Error)`: if there was an error during the database operation
///
/// If the file doesn't exist or has not tags, an empty vec is returned
pub fn get_all_tags_for_file(
    file_id: u32,
    con: &Connection,
) -> Result<Vec<repository::TaggedItem>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/get_all_tags_for_file.sql"
    ))?;
    let rows = pst.query_map(rusqlite::params![file_id], tagged_item_mapper)?;
    let mut tags: Vec<repository::TaggedItem> = Vec::new();
    for tag_res in rows {
        tags.push(tag_res?);
    }
    Ok(tags)
}

pub fn get_tags_for_file(
    file_id: u32,
    tag_type: TagTypes,
    con: &Connection,
) -> Result<Vec<repository::TaggedItem>, rusqlite::Error> {
    let query = match tag_type {
        TagTypes::Explicit => include_str!("../assets/queries/tags/get_explicit_tags_for_file.sql"),
        TagTypes::Implicit => include_str!("../assets/queries/tags/get_implicit_tags_for_file.sql"),
    };
    let mut pst = con.prepare(query)?;
    let rows = pst.query_map(rusqlite::params![file_id], tagged_item_mapper)?;
    rows.collect::<Result<Vec<_>, _>>()
}

/// Retrieves all tags on all files passed, explicit or implied.
/// The returned value is a Map of file id => Vec<[`repository::TaggedItem`]>. Files without _any_ tags will not have an entry in the map
///
/// ## Parameters:
/// - `file_ids` the ids to get tags for
/// - `con` a reference to a database connection. The caller must manage closing the connection.
///
/// ## Returns:
/// - `Ok(HashMap<u32, Vec<repository::TaggedItem>>)` if the tags were successfully retrieved
/// - `Err(rusqlite::Error)` if there was a database error
///
/// ---
/// See also [get_all_tags_on_file]
///
pub fn get_all_tags_for_files(
    file_ids: Vec<u32>,
    con: &Connection,
) -> Result<HashMap<u32, Vec<repository::TaggedItem>>, rusqlite::Error> {
    let in_clause: Vec<String> = file_ids.iter().map(|it| format!("'{it}'")).collect();
    let in_clause = in_clause.join(",");
    let formatted_query = format!(
        include_str!("../assets/queries/tags/get_all_tags_for_files.sql"),
        in_clause
    );
    let mut pst = con.prepare(formatted_query.as_str())?;
    let rows = pst.query_map([], tagged_item_mapper)?;
    let mut mapped: HashMap<u32, Vec<repository::TaggedItem>> = HashMap::new();
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
        "../assets/queries/tags/delete_implicit_tag_from_file.sql"
    ))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

/// Updates or inserts an implicit tag on a file, replacing any existing implicit tag from a different ancestor
pub fn upsert_implicit_tag_to_file(
    tag_id: u32,
    file_id: u32,
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    // First delete any existing implicit tag
    remove_implicit_tag_from_file(tag_id, file_id, con)?;

    // Then insert the new one
    add_implicit_tag_to_file(tag_id, file_id, implicit_from_id, con)
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

/// Retrieves all tags on the folder with the passed id, explicit or implied.
/// If no folder is found, an empty Vec is returned.
///
/// ## Parameters:
/// - `folder_id` the id of the folder in the database to retrieve tags for
/// - `con` a reference to a database connection. The caller must manage closing the connection.
///
/// ## Returns:
/// - `Ok(Vec<repository::TaggedItem>)` if the tags were successfully retrieved
/// - `Err(rusqlite::Error)` if there was a database error
pub fn get_all_tags_for_folder(
    folder_id: u32,
    con: &Connection,
) -> Result<Vec<repository::TaggedItem>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/get_all_tags_for_folder.sql"
    ))?;
    let rows = pst.query_map(rusqlite::params![folder_id], tagged_item_mapper)?;
    rows.collect::<Result<Vec<repository::TaggedItem>, rusqlite::Error>>()
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

/// Deletes an implicit tag from a folder if it exists
pub fn delete_implicit_tag_from_folder(
    tag_id: u32,
    folder_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/delete_implicit_tag_from_folder.sql"
    ))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    Ok(())
}

/// Updates or inserts an implicit tag on a folder, replacing any existing implicit tag from a different ancestor
pub fn upsert_implicit_tag_to_folder(
    tag_id: u32,
    folder_id: u32,
    implicit_from_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    // First delete any existing implicit tag
    delete_implicit_tag_from_folder(tag_id, folder_id, con)?;

    // Then insert the new one
    let mut insert_pst = con.prepare(include_str!(
        "../assets/queries/tags/add_implicit_tag_to_folder.sql"
    ))?;
    insert_pst.execute(rusqlite::params![tag_id, folder_id, implicit_from_id])?;
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

// ================= misc =================
/// 1. id
/// 2. fileId
/// 3. folderId
/// 4. implicitFromId
/// 5. tagId
/// 6. title
fn tagged_item_mapper(row: &rusqlite::Row) -> Result<repository::TaggedItem, rusqlite::Error> {
    let id: u32 = row.get(0)?;
    let file_id: Option<u32> = row.get(1)?;
    let folder_id: Option<u32> = row.get(2)?;
    let implicit_from_id: Option<u32> = row.get(3)?;
    let tag_id: u32 = row.get(4)?;
    let title: String = row.get(5)?;

    Ok(repository::TaggedItem {
        id,
        file_id,
        folder_id,
        implicit_from_id,
        tag_id,
        title,
    })
}

/// maps a [`repository::Tag`] from a database row
fn tag_mapper(row: &rusqlite::Row) -> Result<repository::Tag, rusqlite::Error> {
    let id: u32 = row.get(0)?;
    let title: String = row.get(1)?;
    Ok(repository::Tag { id, title })
}
