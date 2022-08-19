use crate::model::db::FileRecord;
use rusqlite::Connection;

pub fn save_file_record(file: &FileRecord, con: &Connection) -> Result<(), String> {
    //language=sqlite
    let mut pst = con
        .prepare("insert into FileRecords(name, path, hash) values (?1, ?2, ?3); commit;")
        .unwrap();
    let res = match pst.execute((file.name, file.path, file.hash)) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!(
            "Failed to save file record. Nested exception is {:?}",
            e
        )),
    };
    res
}
