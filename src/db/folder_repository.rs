use crate::model::db;
use crate::service::folder_service::LinkFolderError;
use rusqlite::{params, Connection};

pub fn get_by_id(id: u32, con: &Connection) -> Result<db::Folder, rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare(
            "with query as (select fl.id, fl.name, fl.parentId
               from folders fl
               where parentId is null
               union all
               select f.id, query.name || '/' || f.name, f.parentId
               from folders f
                        join query
                             on f.parentId = query.id)
select id, query.name as \"path\", parentId
from query where id = ?1",
        )
        .unwrap();

    Ok(pst.query_row([id], |row| {
        let parent_id: Option<u32> = match row.get(2) {
            Ok(val) => Some(val),
            Err(_) => None,
        };
        Ok(db::Folder {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            parent_id,
        })
    })?)
}

pub fn get_child_folders(id: u32, con: &Connection) -> Result<Vec<db::Folder>, rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare(
            "with query as (select fl.id, fl.name, fl.parentId
               from folders fl
               where parentId is null
               union all
               select f.id, query.name || '/' || f.name, f.parentId
               from folders f
                        join query
                             on f.parentId = query.id)
select query.id, query.name as \"path\", query.parentId
from query
where query.parentId = ?1",
        )
        .unwrap();
    let mut folders = Vec::<db::Folder>::new();
    let mut rows = pst.query([id])?;
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
    id: u32,
    con: &Connection,
) -> Result<Vec<db::FileRecord>, rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare(
            "select f.id, f.name, f.hash from Folder_Files ff
            join FileRecords f on ff.fileId = f.id
            where ff.folderId = ?1",
        )
        .unwrap();
    let mapped = pst.query_map([id], |row| {
        Ok(db::FileRecord {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            hash: row.get(2)?,
            path: None,
        })
    })?;

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
