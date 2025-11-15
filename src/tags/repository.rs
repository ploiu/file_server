use std::{backtrace::Backtrace, collections::HashMap};

use rusqlite::Connection;

use crate::model::repository;

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

/// the caller of this function will need to make sure the tag already exists and isn't already on the file
pub fn add_tag_to_file(file_id: u32, tag_id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/add_tag_to_file.sql"))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

pub fn get_tags_on_file(
    file_id: u32,
    con: &Connection,
) -> Result<Vec<repository::Tag>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/get_tags_for_file.sql"))?;
    let rows = pst.query_map(rusqlite::params![file_id], tag_mapper)?;
    let mut tags: Vec<repository::Tag> = Vec::new();
    for tag_res in rows {
        tags.push(tag_res?);
    }
    Ok(tags)
}

pub fn get_tags_on_files(
    file_ids: Vec<u32>,
    con: &Connection,
) -> Result<HashMap<u32, Vec<repository::Tag>>, rusqlite::Error> {
    struct TagFile {
        file_id: u32,
        tag_id: u32,
        tag_title: String,
    }
    let in_clause: Vec<String> = file_ids.iter().map(|it| format!("'{it}'")).collect();
    let in_clause = in_clause.join(",");
    let formatted_query = format!(
        include_str!("../assets/queries/tags/get_tags_for_files.sql"),
        in_clause
    );
    let mut pst = con.prepare(formatted_query.as_str())?;
    let rows = pst.query_map([], |row| {
        let file_id: u32 = row.get(0)?;
        let tag_id: u32 = row.get(1)?;
        let tag_title: String = row.get(2)?;
        Ok(TagFile {
            file_id,
            tag_id,
            tag_title,
        })
    })?;
    let mut mapped: HashMap<u32, Vec<repository::Tag>> = HashMap::new();
    for res in rows {
        let res = res?;
        if let std::collections::hash_map::Entry::Vacant(e) = mapped.entry(res.file_id) {
            e.insert(vec![repository::Tag {
                id: res.tag_id,
                title: res.tag_title,
            }]);
        } else {
            mapped.get_mut(&res.file_id).unwrap().push(repository::Tag {
                id: res.tag_id,
                title: res.tag_title,
            });
        }
    }
    Ok(mapped)
}

pub fn remove_tag_from_file(
    file_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_tag_from_file.sql"
    ))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

pub fn add_tag_to_folder(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/add_tag_to_folder.sql"))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    Ok(())
}

pub fn get_tags_on_folder(
    folder_id: u32,
    con: &Connection,
) -> Result<Vec<repository::Tag>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/get_tags_for_folder.sql"
    ))?;
    let rows = pst.query_map(rusqlite::params![folder_id], |row| Ok(tag_mapper(row)))?;
    let mut tags: Vec<repository::Tag> = Vec::new();
    for tag_res in rows {
        // I know it's probably bad style, but I'm laughing too hard at the double question mark.
        // no I don't know what my code is doing and I'm glad my code reflects that
        tags.push(tag_res??);
    }
    Ok(tags)
}

pub fn remove_tag_from_folder(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_tag_from_folder.sql"
    ))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    Ok(())
}

fn tag_mapper(row: &rusqlite::Row) -> Result<repository::Tag, rusqlite::Error> {
    let id: u32 = row.get(0)?;
    let title: String = row.get(1)?;
    Ok(repository::Tag { id, title })
}
