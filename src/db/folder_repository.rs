use crate::model::db;
use rusqlite::Connection;
use std::any::Any;

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
            Err(e) => None,
        };
        Ok(db::Folder {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            parent_id,
        })
    })?)
}
