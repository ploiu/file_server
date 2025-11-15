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
    // Check if folder already has this tag (either direct or inherited) BEFORE making any changes
    let already_had_tag = {
        let mut check_pst = con.prepare(
            "SELECT 1 FROM TaggedItems WHERE folderId = ?1 AND tagId = ?2 LIMIT 1"
        )?;
        check_pst.exists(rusqlite::params![folder_id, tag_id])?
    };
    
    // Insert or update the tag to be direct
    let mut pst = con.prepare(include_str!("../assets/queries/tags/add_tag_to_folder.sql"))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    
    // Only propagate to descendants if the folder didn't already have this tag
    // If it had the tag (direct or inherited), descendants already have it too
    if !already_had_tag {
        add_inherited_tag_to_descendants(folder_id, tag_id, con)?;
    }
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
    // Remove inherited tags from descendants
    remove_inherited_tag_from_descendants(folder_id, tag_id, con)?;
    // Re-establish inheritance from higher ancestors if applicable
    reinherit_tag_for_descendants(folder_id, tag_id, con)?;
    Ok(())
}

/// Automatically inherit tags from all ancestor folders for a specific file.
/// This should be called when a file is created or moved to a new folder.
pub fn inherit_tags_for_file(file_id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/inherit_tags_for_file.sql"
    ))?;
    pst.execute(rusqlite::params![file_id])?;
    Ok(())
}

/// Automatically inherit tags from all ancestor folders for a specific folder.
/// This should be called when a folder is created or moved to a new parent.
pub fn inherit_tags_for_folder(folder_id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/inherit_tags_for_folder.sql"
    ))?;
    pst.execute(rusqlite::params![folder_id])?;
    Ok(())
}

/// Add inherited tag entries to all descendants when a tag is directly added to a folder.
/// This propagates the tag down the folder hierarchy.
pub fn add_inherited_tag_to_descendants(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    // Add to descendant folders
    let folder_query = r"
        with recursive descendants(folderId) as (
            select id from Folders where parentId = ?
            union all
            select f.id
            from Folders f
            join descendants d on f.parentId = d.folderId
        )
        insert into TaggedItems(tagId, folderId, inheritedFromId)
        select DISTINCT ?, d.folderId, ?
        from descendants d
        where not exists (
            select 1 from TaggedItems ti 
            where ti.tagId = ? and ti.folderId = d.folderId
        )";
    
    con.execute(folder_query, rusqlite::params![folder_id, tag_id, folder_id, tag_id])?;
    
    // Add to descendant files
    let file_query = r"
        with recursive descendants(folderId) as (
            select ? as folderId
            union all
            select id from Folders where parentId = ?
            union all
            select f.id
            from Folders f
            join descendants d on f.parentId = d.folderId
        )
        insert into TaggedItems(tagId, fileId, inheritedFromId)
        select DISTINCT ?, ff.fileId, ?
        from descendants d
        join Folder_Files ff on ff.folderId = d.folderId
        where not exists (
            select 1 from TaggedItems ti 
            where ti.tagId = ? and ti.fileId = ff.fileId
        )";
    
    con.execute(file_query, rusqlite::params![folder_id, folder_id, tag_id, folder_id, tag_id])?;
    
    Ok(())
}

/// Remove inherited tag entries from descendants when a direct tag is removed from a folder.
/// Only removes tags that were inherited from this specific folder.
pub fn remove_inherited_tag_from_descendants(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let query = include_str!("../assets/queries/tags/remove_inherited_tag_from_descendants.sql");
    con.execute_batch(&query.replace("?1", &folder_id.to_string()).replace("?2", &tag_id.to_string()))?;
    Ok(())
}

/// Re-establish inheritance for descendants that might inherit the tag from a higher ancestor.
/// Called after removing a direct tag from a folder.
pub fn reinherit_tag_for_descendants(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let query = include_str!("../assets/queries/tags/reinherit_tag_for_descendants.sql");
    con.execute_batch(&query.replace("?1", &folder_id.to_string()).replace("?2", &tag_id.to_string()))?;
    Ok(())
}

fn tag_mapper(row: &rusqlite::Row) -> Result<repository::Tag, rusqlite::Error> {
    let id: u32 = row.get(0)?;
    let title: String = row.get(1)?;
    Ok(repository::Tag { id, title })
}
