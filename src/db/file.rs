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
