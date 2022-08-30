use rusqlite::Connection;

use crate::model::db::FileRecord;

pub fn save_file_record(
    file: &FileRecord,
    // folder_file: &FolderFiles,
    con: &Connection,
) -> Result<u32, String> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/create_file.sql"))
        .unwrap();
    let res = match pst.insert((file.name.as_str(), file.hash.as_str())) {
        Ok(id) => Ok(id as u32),
        Err(e) => {
            return Err(format!(
                "Failed to save file record. Nested exception is {:?}",
                e
            ))
        }
    };
    res
}

pub fn get_by_id(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/get_file_by_id.sql"))
        .unwrap();

    Ok(pst.query_row([id], |row| {
        Ok(FileRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            hash: row.get(2)?,
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
pub fn delete_by_id(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/delete_file_by_id.sql"))
        .unwrap();

    // we need to be able to delete the file off the disk, so we have to return the FileRecord too
    let record = match get_by_id(id, &con) {
        Ok(f) => f,
        Err(e) => return Err(e),
    };

    match pst.execute([id]) {
        Err(e) => return Err(e),
        // we don't do anything here because we don't care - no rows deleted means we throw an error via get_by_id
        _ => {}
    };
    return Ok(record);
}
