use rusqlite::Connection;

use crate::model::db::FileRecord;

pub fn save_file_record(file: &FileRecord, con: &Connection) -> Result<(), String> {
    //language=sqlite
    let mut pst = con
        .prepare("insert into FileRecords(name, path, hash) values (?1, ?2, ?3); commit;")
        .unwrap();
    let res = match pst.execute((file.name.as_str(), file.path.as_str(), file.hash.as_str())) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!(
            "Failed to save file record. Nested exception is {:?}",
            e
        )),
    };
    res
}

pub fn get_by_id(id: u64, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare("select id, name, path, hash from FileRecords where id = ?1")
        .unwrap();

    Ok(pst.query_row([id], |row| {
        Ok(FileRecord {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            path: row.get(2)?,
            hash: row.get(3)?,
        })
    })?)
}

pub fn delete_by_id(id: u64, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    //language=sqlite
    let mut pst = con
        .prepare("delete from FileRecords where id = ?1")
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
