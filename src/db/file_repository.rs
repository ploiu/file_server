use rusqlite::Connection;

use crate::model::db::FileRecord;

pub fn save_file_record(
    file: &FileRecord,
    // folder_file: &FolderFiles,
    con: &Connection,
) -> Result<u32, String> {
    //language=sqlite
    let mut pst = con
        .prepare("insert into FileRecords(name, hash) values (?1, ?2); commit;")
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
    //language=sqlite
    let mut pst = con
        .prepare("select id, name, hash from FileRecords where id = ?1")
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
select coalesce(query.name || '/', '') || f.name from FileRecords f
left join Folder_Files FF on f.id = FF.fileId
left join query on query.id = ff.folderId
where f.id = ?1",
        )
        .unwrap();
    let result = pst.query_row([id], |row| Ok(row.get(0)?));
    return result;
}

/// removes the file with the passed id from the database
pub fn delete_by_id(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
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