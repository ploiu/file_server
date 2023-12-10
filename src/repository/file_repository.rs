use std::collections::HashSet;

use rusqlite::{params, Connection};

use crate::model::repository::FileRecord;

pub fn create_file(file: &FileRecord, con: &Connection) -> Result<u32, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/create_file.sql"))
        .unwrap();

    match pst.insert(params![file.name]) {
        Ok(id) => Ok(id as u32),
        Err(e) => {
            eprintln!("Failed to save file record. Nested exception is {:?}", e);
            Err(e)
        }
    }
}

pub fn get_file(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/get_file_by_id.sql"))
        .unwrap();

    pst.query_row([id], map_file)
}

/// returns the full path (excluding root name) of the specified file in the database
pub fn get_file_path(id: u32, con: &Connection) -> Result<String, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/file/get_file_path_by_id.sql"
        ))
        .unwrap();
    pst.query_row([id], |row| row.get(0))
}

/// removes the file with the passed id from the database
pub fn delete_file(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/delete_file_by_id.sql"))
        .unwrap();

    // we need to be able to delete the file off the disk, so we have to return the FileRecord too
    let record = get_file(id, con)?;

    if let Err(e) = pst.execute([id]) {
        eprintln!("Failed to delete file by id. Nested exception is {:?}", e);
        return Err(e);
    }
    Ok(record)
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
    let rows = pst.query_map([&criteria], map_file)?;
    for file in rows.into_iter() {
        results.push(file?);
    }
    Ok(results)
}

pub fn map_file(row: &rusqlite::Row) -> Result<FileRecord, rusqlite::Error> {
    let id = row.get(0)?;
    let name = row.get(1)?;
    let parent_id = row.get(2)?;
    Ok(FileRecord {
        id,
        name,
        parent_id,
    })
}

pub fn get_files_by_all_tags(
    tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<FileRecord>, rusqlite::Error> {
    let base_sql_string = include_str!("../assets/queries/file/get_files_by_all_tags.sql");
    // need to fill out the in clause and the count clause
    let joined_tags = tags
        .iter()
        .map(|t| format!("'{}'", t.replace("'", "''")))
        .reduce(|combined, current| format!("{combined},{current}"))
        .unwrap();
    let replaced_string = base_sql_string
        .to_string()
        .replace("?1", joined_tags.as_str())
        .replace("?2", tags.len().to_string().as_str());
    let mut pst = con.prepare(replaced_string.as_str())?;
    let mut files: HashSet<FileRecord> = HashSet::new();
    let res = pst.query_map([], map_file)?;
    for file in res {
        files.insert(file?);
    }
    Ok(files)
}

#[cfg(test)]
mod get_files_by_all_tags_tests {
    use std::collections::HashSet;

    use rusqlite::Connection;

    use crate::model::repository::FileRecord;
    use crate::repository::file_repository::get_files_by_all_tags;
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_file_db_entry, create_tag_files, refresh_db};

    #[test]
    fn returns_files_with_all_tags() {
        refresh_db();
        let con: Connection = open_connection();
        create_file_db_entry("bad", None);
        create_file_db_entry("has some", None); // 2
        create_file_db_entry("has all", None); // 3
        create_file_db_entry("also has all", None); // 4
                                                    // add tags
        create_tag_files("tag1", vec![2, 3, 4]);
        create_tag_files("asdf", vec![3, 4]);
        create_tag_files("fda", vec![2, 3, 4]);

        let res = get_files_by_all_tags(
            &HashSet::from(["tag1".to_string(), "fda".to_string(), "asdf".to_string()]),
            &con,
        )
        .unwrap()
        .into_iter()
        .collect::<Vec<FileRecord>>();
        con.close().unwrap();
        assert_eq!(2, res.len());
        assert!(res.contains(&FileRecord {
            id: Some(3),
            name: "has all".to_string(),
            parent_id: None,
        }));
        assert!(res.contains(&FileRecord {
            id: Some(4),
            name: "also has all".to_string(),
            parent_id: None,
        }));
        cleanup();
    }
}
