use rusqlite::{params, Connection};

use crate::model::repository::FileRecord;

pub fn create_file(file: &FileRecord, con: &Connection) -> Result<u32, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/create_file.sql"))
        .unwrap();
    let res = match pst.insert(params![file.name]) {
        Ok(id) => Ok(id as u32),
        Err(e) => {
            eprintln!("Failed to save file record. Nested exception is {:?}", e);
            return Err(e);
        }
    };
    res
}

pub fn get_file(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/get_file_by_id.sql"))
        .unwrap();

    Ok(pst.query_row([id], |row| {
        Ok(FileRecord {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?)
}

/// returns the full path (excluding root name) of the specified file in the database
pub fn get_file_path(id: u32, con: &Connection) -> Result<String, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/file/get_file_path_by_id.sql"
        ))
        .unwrap();
    let result = pst.query_row([id], |row| Ok(row.get(0)?));
    return result;
}

/// removes the file with the passed id from the database
pub fn delete_file(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/delete_file_by_id.sql"))
        .unwrap();

    // we need to be able to delete the file off the disk, so we have to return the FileRecord too
    let record = get_file(id, &con)?;

    if let Err(e) = pst.execute([id]) {
        eprintln!("Failed to delete file by id. Nested exception is {:?}", e);
        return Err(e);
    }
    return Ok(record);
}

/// renames the file with the passed id and links it to the folder with the passed id in the database.
/// This performs no checks, so file name and paths must be checked ahead of time
pub fn update_file(
    file_id: &u32,
    parent_id: &Option<u32>,
    file_name: &String,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut update_name_pst = con
        .prepare(include_str!("../assets/queries/file/rename_file.sql"))
        .unwrap();
    let mut unlink_file_pst = con
        .prepare(include_str!(
            "../assets/queries/folder_file/delete_folder_file_by_file_id.sql"
        ))
        .unwrap();
    // now to rename the file
    update_name_pst.execute(rusqlite::params![file_id, file_name])?;
    unlink_file_pst.execute([file_id])?;
    // if we specified a parent id, we need to add a link back
    if let Some(parent_id) = parent_id {
        let mut add_link_pst = con
            .prepare(include_str!(
                "../assets/queries/folder_file/create_folder_file.sql"
            ))
            .unwrap();
        add_link_pst.execute([file_id, parent_id])?;
    }
    Ok(())
}

/// performs a fuzzy search using the passed criteria.
/// The fuzzy search mashes all the fields together and performs a sql `LIKE` clause on the input
pub fn search_files(
    criteria: String,
    con: &Connection,
) -> Result<Vec<FileRecord>, rusqlite::Error> {
    let criteria = format!("%{}%", criteria);
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/search_files.sql"))
        .unwrap();
    let mut results: Vec<FileRecord> = Vec::new();
    let rows = pst.query_map([&criteria], |row| {
        Ok(FileRecord {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    for file in rows.into_iter() {
        results.push(file?);
    }
    Ok(results)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::repository;
    use crate::repository::DB_LOCATION;
    use crate::service::file_service::FILE_DIR;
    use std::fs;

    fn before_each() {
        fs::remove_dir_all(FILE_DIR);
        fs::remove_file(DB_LOCATION);
        repository::initialize_db().unwrap();
    }

    #[test]
    fn test_create_get_file() {
        before_each();
        let con = repository::open_connection();
        let record = FileRecord {
            id: None,
            name: String::from("test.txt"),
        };
        let id = create_file(&record, &con).unwrap();
        let file = get_file(id, &con).unwrap();
        con.close().unwrap();
        assert_eq!(file.id.unwrap(), id);
        assert_eq!(file.name, record.name);
    }

    #[test]
    fn test_delete_get_file() {
        before_each();
        let con = repository::open_connection();
        let record = FileRecord {
            id: None,
            name: String::from("test.txt"),
        };
        let id = create_file(&record, &con).unwrap();
        delete_file(id, &con).unwrap();
        let res = get_file(id, &con).unwrap_err();
        con.close().unwrap();
        assert_eq!(res, rusqlite::Error::QueryReturnedNoRows);
    }

    #[test]
    fn test_update_get_file() {
        before_each();
        let con = repository::open_connection();
        let record = FileRecord {
            id: None,
            name: String::from("test.txt"),
        };
        let id = create_file(&record, &con).unwrap();
        update_file(&id, &None, &String::from("updated.txt"), &con).unwrap();
        let file = get_file(id, &con).unwrap();
        con.close().unwrap();
        assert_eq!(file.id.unwrap(), id);
        assert_eq!(file.name, "updated.txt")
    }
}
