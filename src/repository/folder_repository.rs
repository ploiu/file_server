use crate::model::repository;
use rusqlite::{params, Connection, Row, Rows};

pub fn get_by_id(id: Option<u32>, con: &Connection) -> Result<repository::Folder, rusqlite::Error> {
    // if id is none, we're talking about the root folder
    if id.is_none() {
        return Ok(repository::Folder {
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
        Ok(repository::Folder {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            parent_id: row.get(2).ok().or_else(|| None),
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
) -> Result<Vec<repository::Folder>, rusqlite::Error> {
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
    let mut folders = Vec::<repository::Folder>::new();
    let mut rows: Rows;
    if id.is_some() {
        rows = pst.query([id.unwrap()])?;
    } else {
        rows = pst.query([])?;
    }
    while let Some(row) = rows.next()? {
        // these folders are guaranteed to have a parent folder id
        folders.push(repository::Folder {
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
pub fn create_folder(
    folder: &repository::Folder,
    con: &Connection,
) -> Result<repository::Folder, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/folder/create_folder.sql"))
        .unwrap();
    return match folder.parent_id {
        Some(id) => {
            let folder_id = pst.insert(rusqlite::params![folder.name, id])? as u32;
            Ok(repository::Folder {
                id: Some(folder_id),
                name: String::from(&folder.name),
                parent_id: folder.parent_id,
            })
        }
        None => {
            let folder_id =
                pst.insert(rusqlite::params![folder.name, rusqlite::types::Null])? as u32;
            Ok(repository::Folder {
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
pub fn update_folder(folder: &repository::Folder, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/folder/update_folder.sql"))
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
) -> Result<Vec<repository::FileRecord>, rusqlite::Error> {
    let mut pst = if id.is_some() {
        con.prepare(include_str!(
            "../assets/queries/folder_file/get_child_files_with_id.sql"
        ))
        .unwrap()
    } else {
        con.prepare(include_str!(
            "../assets/queries/file/get_child_files_without_id.sql"
        ))
        .unwrap()
    };
    let row_mapper = |row: &Row| {
        Ok(repository::FileRecord {
            id: Some(row.get(0)?),
            name: row.get(1)?,
        })
    };
    let mapped = if id.is_some() {
        pst.query_map([id.unwrap()], row_mapper)?
    } else {
        pst.query_map([], row_mapper)?
    };

    let mut files: Vec<repository::FileRecord> = Vec::new();
    for file in mapped.into_iter() {
        files.push(file?);
    }
    Ok(files)
}

/// deletes a folder and unlinks every file inside of it.
/// This _does not_ check if the folder exists first.
pub fn delete_folder(id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut delete_folder_files = con
        .prepare(include_str!(
            "../assets/queries/folder_file/delete_folder_file_by_folder_id.sql"
        ))
        .unwrap();
    let mut delete_folder = con
        .prepare(include_str!(
            "../assets/queries/folder/delete_folder_by_id.sql"
        ))
        .unwrap();
    delete_folder_files.execute([id])?;
    delete_folder.execute([id])?;
    Ok(())
}

pub fn link_folder_to_file(
    file_id: u32,
    folder_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/folder_file/create_folder_file.sql"
        ))
        .unwrap();
    return match pst.insert([file_id, folder_id]) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to link file to folder. Nested exception is {:?}", e);
            return Err(e);
        }
    };
}

/// returns all the ids of all child folders
pub fn get_all_child_folder_ids(id: u32, con: &Connection) -> Result<Vec<u32>, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/folder/get_child_folder_ids_recursive.sql"
        ))
        .unwrap();
    let mut ids = Vec::<u32>::new();
    let res = pst.query_map([id], |row| Ok(row.get(0)?))?;
    for i in res.into_iter() {
        ids.push(i.unwrap());
    }
    Ok(ids)
}
