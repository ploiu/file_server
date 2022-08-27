use crate::model::db;
use rusqlite::{params, Connection, Row, Rows};

pub fn get_by_id(id: Option<u32>, con: &Connection) -> Result<db::Folder, rusqlite::Error> {
    // if id is none, we're talking about the root folder
    if id.is_none() {
        return Ok(db::Folder {
            id: Some(0), // will never collide with an id since sqlite starts with 1
            name: String::from("root"),
            parent_id: None,
        });
    }
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/folder/get_folder_by_id.sql"
        ))
        .unwrap();

    let row_mapper = |row: &Row| {
        let parent_id: Option<u32> = match row.get(2) {
            Ok(val) => Some(val),
            Err(_) => None,
        };
        Ok(db::Folder {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            parent_id,
        })
    };

    return match id {
        Some(id) => Ok(pst.query_row([id], row_mapper)?),
        None => Ok(pst.query_row([rusqlite::types::Null], row_mapper)?),
    };
}

pub fn get_child_folders(
    id: Option<u32>,
    con: &Connection,
) -> Result<Vec<db::Folder>, rusqlite::Error> {
    let mut pst = if id.is_some() {
        con.prepare(include_str!(
            "../assets/queries/folder/get_child_folders_with_id.sql"
        ))
        .unwrap()
    } else {
        con.prepare(include_str!(
            "../assets/queries/folder/get_child_folders_without_id.sql"
        ))
        .unwrap()
    };
    let mut folders = Vec::<db::Folder>::new();
    let mut rows: Rows;
    if id.is_some() {
        rows = pst.query([id.unwrap()])?;
    } else {
        rows = pst.query([])?;
    }
    while let Some(row) = rows.next()? {
        // these folders are guaranteed to have a parent folder id
        folders.push(db::Folder {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            parent_id: row.get(2)?,
        })
    }
    Ok(folders)
}

/// creates a folder record in the database.
/// This does not do any checks on folder parent id or any other data,
/// and that must be done before this function is called
pub fn create_folder(folder: &db::Folder, con: &Connection) -> Result<db::Folder, rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare("insert into Folders(name, parentId) values(?1, ?2)")
        .unwrap();
    return match folder.parent_id {
        Some(id) => {
            let folder_id = pst.insert(rusqlite::params![folder.name, id])? as u32;
            Ok(db::Folder {
                id: Some(folder_id),
                name: String::from(&folder.name),
                parent_id: folder.parent_id,
            })
        }
        None => {
            let folder_id =
                pst.insert(rusqlite::params![folder.name, rusqlite::types::Null])? as u32;
            Ok(db::Folder {
                id: Some(folder_id),
                name: String::from(&folder.name),
                parent_id: folder.parent_id,
            })
        }
    };
}

/// updates a folder record in the database.
/// This does not perform any checks on folder info, and that must be done
/// before this function is called
pub fn update_folder(folder: &db::Folder, con: &Connection) -> Result<(), rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare(
            "update Folders
                    set name = ?1, parentId = ?2
                    where id = ?3",
        )
        .unwrap();
    match folder.parent_id {
        Some(parent_id) => pst.execute(params![&folder.name, parent_id, &folder.id])?,
        // moving to top level
        None => pst.execute(params![&folder.name, rusqlite::types::Null, &folder.id])?,
    };
    Ok(())
}

/// retrieves all the
pub fn get_files_for_folder(
    id: Option<u32>,
    con: &Connection,
) -> Result<Vec<db::FileRecord>, rusqlite::Error> {
    let mut pst = if id.is_some() {
        con.prepare(include_str!(
            "../assets/queries/file/get_child_files_with_id.sql"
        ))
        .unwrap()
    } else {
        con.prepare(include_str!(
            "../assets/queries/file/get_child_files_without_id.sql"
        ))
        .unwrap()
    };
    let row_mapper = |row: &Row| {
        Ok(db::FileRecord {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            hash: row.get(2)?,
        })
    };
    let mapped = if id.is_some() {
        pst.query_map([id.unwrap()], row_mapper)?
    } else {
        pst.query_map([], row_mapper)?
    };

    let mut files: Vec<db::FileRecord> = Vec::new();
    for file in mapped.into_iter() {
        files.push(file?);
    }
    Ok(files)
}

/// deletes a folder and unlinks every file inside of it.
/// This _does not_ check if the folder exists first.
pub fn delete_folder(id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    //language=sqlite
    let mut delete_folder_files = con
        .prepare("delete from Folder_Files where folderId = ?1")
        .unwrap();
    //language=sqlite
    let mut delete_folder = con.prepare("delete from Folders where id = ?1").unwrap();
    delete_folder_files.execute([id])?;
    delete_folder.execute([id])?;
    Ok(())
}

pub fn link_folder_to_file(
    file_id: u32,
    folder_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare("insert into Folder_Files(fileId, folderId) values (?1, ?2)")
        .unwrap();
    return match pst.insert([file_id, folder_id]) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!(
                "Failed to link file to folder. Nested exception is: \n {:?}",
                e
            );
            return Err(e);
        }
    };
}

/// returns all the ids of all child folders
pub fn get_all_child_folder_ids(id: u32, con: &Connection) -> Result<Vec<u32>, rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare(
            // recursively retrieve all IDs of a given folder
            "with query as (select f1.id
               from folders f1
               where f1.parentId = ?1
               union all
               select f2.id
               from folders f2

                        join query on f2.parentId = query.id)
select id
from query;
",
        )
        .unwrap();
    let mut ids = Vec::<u32>::new();
    let res = pst.query_map([id], |row| {
        let i: u32 = row.get(0)?;
        Ok(i)
    })?;
    for i in res.into_iter() {
        ids.push(i.unwrap());
    }
    Ok(ids)
}
