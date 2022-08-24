use crate::model::db;
use rusqlite::Connection;

pub fn get_by_id(id: u64, con: &Connection) -> Result<db::Folder, rusqlite::Error> {
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

pub fn get_child_folders(id: u64, con: &Connection) -> Result<Vec<db::Folder>, rusqlite::Error> {
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
