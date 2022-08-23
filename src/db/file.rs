use rusqlite::Connection;
use std::any::{Any, TypeId};

use crate::model::db::FileRecord;

pub fn save_file_record(file: &FileRecord, con: &Connection) -> Result<(), String> {
    //language=sqlite
    let mut pst = con
        .prepare("insert into FileRecords(name, hash) values (?1, ?3); commit;")
        .unwrap();
    let res = match pst.execute((file.name.as_str(), file.hash.as_str())) {
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
        .prepare(
            "with query as (select fl.id, fl.name, fl.parentId
               from folders fl
               where parentId is null
               union all
               select f.id, query.name || '/' || f.name, f.parentId
               from folders f
                        join query
                             on f.parentId = query.id)
select FR.id, FR.name, FR.hash, query.name as \"path\"
from query
         join Folder_Files ff on ff.folderId = query.id
         join FileRecords FR on ff.fileId = FR.id
where fr.id = ?1",
        )
        .unwrap();

    Ok(pst.query_row([id], |row| {
        Ok(FileRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            hash: row.get(2)?,
            path: row.get(3)?,
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
