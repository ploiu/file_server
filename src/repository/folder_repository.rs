use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, Rows};

use crate::model::repository;
use crate::repository::file_repository;

pub fn get_by_id(id: Option<u32>, con: &Connection) -> Result<repository::Folder, rusqlite::Error> {
    // if id is none, we're talking about the root folder
    if id.is_none() || id == Some(0) {
        return Ok(repository::Folder {
            id: Some(0), // will never collide with an id since sqlite starts with 1
            name: String::from("root"),
            parent_id: None,
        });
    }
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/folder/get_folder_by_id.sql"
        ))
        .unwrap();

    match id {
        Some(id) => Ok(pst.query_row([id], map_folder)?),
        None => Ok(pst.query_row([rusqlite::types::Null], map_folder)?),
    }
}

pub fn get_child_folders(
    id: Option<u32>,
    con: &Connection,
) -> Result<Vec<repository::Folder>, rusqlite::Error> {
    let mut pst = if id.is_some() {
        con.prepare(include_str!(
            "../assets/queries/folder/get_child_folders_with_id.sql"
        ))
        .unwrap()
    } else {
        con.prepare(include_str!(
            "../assets/queries/folder/get_child_folders_without_id.sql"
        ))
        .unwrap()
    };
    let mut folders = Vec::<repository::Folder>::new();
    let mut rows: Rows;
    if id.is_some() {
        rows = pst.query([id.unwrap()])?;
    } else {
        rows = pst.query([])?;
    }
    while let Some(row) = rows.next()? {
        folders.push(map_folder(row)?)
    }
    Ok(folders)
}

/// creates a folder record in the database.
/// This does not do any checks on folder parent id or any other data,
/// and that must be done before this function is called
pub fn create_folder(
    folder: &repository::Folder,
    con: &Connection,
) -> Result<repository::Folder, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/folder/create_folder.sql"))
        .unwrap();
    match folder.parent_id {
        Some(id) => {
            let folder_id = pst.insert(rusqlite::params![folder.name, id])? as u32;
            Ok(repository::Folder {
                id: Some(folder_id),
                name: String::from(&folder.name),
                parent_id: folder.parent_id,
            })
        }
        None => {
            let folder_id =
                pst.insert(rusqlite::params![folder.name, rusqlite::types::Null])? as u32;
            Ok(repository::Folder {
                id: Some(folder_id),
                name: String::from(&folder.name),
                parent_id: folder.parent_id,
            })
        }
    }
}

/// updates a folder record in the database.
/// This does not perform any checks on folder info, and that must be done
/// before this function is called
pub fn update_folder(folder: &repository::Folder, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/folder/update_folder.sql"))
        .unwrap();
    match folder.parent_id {
        Some(parent_id) => pst.execute(params![&folder.name, parent_id, &folder.id])?,
        // moving to top level
        None => pst.execute(params![&folder.name, rusqlite::types::Null, &folder.id])?,
    };
    Ok(())
}

pub fn get_child_files<T: IntoIterator<Item = u32> + Clone>(
    ids: T,
    con: &Connection,
) -> Result<Vec<repository::FileRecord>, rusqlite::Error> {
    // `is_empty` is not part of a trait, so we have to convert ids
    let ids: HashSet<u32> = ids.clone().into_iter().collect();
    if ids.is_empty() {
        get_child_files_root(con)
    } else {
        get_child_files_non_root(ids, con)
    }
}

/// deletes a folder and unlinks every file inside of it.
/// This _does not_ check if the folder exists first.
pub fn delete_folder(id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut delete_folder_files = con
        .prepare(include_str!(
            "../assets/queries/folder_file/delete_folder_file_by_folder_id.sql"
        ))
        .unwrap();
    let mut delete_folder = con
        .prepare(include_str!(
            "../assets/queries/folder/delete_folder_by_id.sql"
        ))
        .unwrap();
    delete_folder_files.execute([id])?;
    delete_folder.execute([id])?;
    Ok(())
}

pub fn link_folder_to_file(
    file_id: u32,
    folder_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/folder_file/create_folder_file.sql"
        ))
        .unwrap();
    match pst.insert([file_id, folder_id]) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to link file to folder. Nested exception is {:?}", e);
            Err(e)
        }
    }
}

/// returns all the ids of all child folders
pub fn get_all_child_folder_ids<T: IntoIterator<Item = u32> + Clone>(
    input_ids: &T,
    con: &Connection,
) -> Result<Vec<u32>, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/folder/get_child_folder_ids_recursive.sql"
        ))
        .unwrap();
    let input_ids: HashSet<u32> = input_ids.clone().into_iter().collect();
    let mut ids: Vec<u32> = Vec::new();
    let joined_ids = if input_ids.is_empty() {
        String::new()
    } else {
        input_ids
            .into_iter()
            .map(|id| { format!("{id}") }.to_string())
            .reduce(|combined, current| format!("{combined},{current}"))
            .unwrap()
    };
    let res = pst.query_map([joined_ids], |row| row.get(0))?;
    for i in res.into_iter() {
        ids.push(i.unwrap());
    }
    Ok(ids)
}

pub fn get_folders_by_any_tag(
    tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<repository::Folder>, rusqlite::Error> {
    // TODO look at rarray to pass a collection as a parameter (https://docs.rs/rusqlite/0.29.0/rusqlite/vtab/array/index.html)
    let joined_tags = tags
        .iter()
        .map(|t| format!("'{}'", t.replace('\'', "''")))
        .reduce(|combined, current| format!("{combined},{current}"))
        .unwrap();
    let query = include_str!("../assets/queries/folder/get_folders_by_any_tag.sql");
    let replaced_query = query.replace("?1", joined_tags.as_str());
    let mut pst = con.prepare(replaced_query.as_str()).unwrap();
    let mut folders: HashSet<repository::Folder> = HashSet::new();
    let rows = pst.query_map([], map_folder)?;
    for row in rows {
        folders.insert(row?);
    }
    Ok(folders)
}

pub fn get_parent_folders_by_tag<'a, T: IntoIterator<Item = &'a String> + Clone>(
    folder_id: u32,
    tags: &T,
    con: &Connection,
) -> Result<HashMap<u32, HashSet<String>>, rusqlite::Error> {
    let query = include_str!("../assets/queries/folder/get_parent_folders_with_tags.sql");
    // because I'm not using a rusqlite extension, I have to join the list of tags manually
    let joined_tags = tags
        .clone()
        .into_iter()
        .map(|t| format!("'{}'", t.replace('\'', "''")))
        .reduce(|combined, current| format!("{combined},{current}"))
        .unwrap();
    let built_query = query.replace("?2", joined_tags.as_str());
    let mut pst = con.prepare(built_query.as_str())?;
    let mut pairs: HashMap<u32, HashSet<String>> = HashMap::new();
    let mut rows = pst.query([folder_id])?;
    while let Some(row) = rows.next()? {
        let folder_id: u32 = row.get(0)?;
        let tags: String = row.get(1)?;
        let split_tags = tags
            .split(',')
            .map(|s| s.to_string())
            .collect::<HashSet<String>>();
        pairs.insert(folder_id, split_tags);
    }
    Ok(pairs)
}

fn map_folder(row: &rusqlite::Row) -> Result<repository::Folder, rusqlite::Error> {
    let id: Option<u32> = row.get(0)?;
    let name: String = row.get(1)?;
    let parent_id: Option<u32> = row.get(2)?;
    Ok(repository::Folder {
        id,
        name,
        parent_id,
    })
}

fn get_child_files_root(con: &Connection) -> Result<Vec<repository::FileRecord>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/file/get_child_files_without_id.sql"
    ))?;
    let mapped = pst.query_map([], file_repository::map_file_all_fields)?;
    let mut files: Vec<repository::FileRecord> = Vec::new();
    for file in mapped.into_iter() {
        files.push(file?);
    }
    Ok(files)
}

fn get_child_files_non_root(
    ids: HashSet<u32>,
    con: &Connection,
) -> Result<Vec<repository::FileRecord>, rusqlite::Error> {
    let query_string = include_str!("../assets/queries/folder_file/get_child_files_with_id.sql");
    // can't pass a collection of values for a single parameter, and can't combine them and pass as a string param because rusqlite wraps it in '' which we don't want for numeric IDs
    let joined_ids = ids
        .into_iter()
        .map(|id| id.to_string())
        .reduce(|combined, current| format!("{combined}, {current}"))
        .expect("get_child_files_with_id: failed to reduce id collection");
    let query_string = query_string.replace("?1", joined_ids.as_str());
    let mut pst = con.prepare(query_string.as_str())?;
    let mapped = pst.query_map([], file_repository::map_file_all_fields)?;
    let mut files: Vec<repository::FileRecord> = Vec::new();
    for file in mapped.into_iter() {
        files.push(file?);
    }
    Ok(files)
}

#[cfg(test)]
mod get_folders_by_any_tag_tests {
    use std::collections::HashSet;

    use rusqlite::Connection;

    use crate::model::repository::Folder;
    use crate::repository::folder_repository::get_folders_by_any_tag;
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_folder_db_entry, create_tag_folders, refresh_db};

    #[test]
    fn returns_folders_with_any_tag() {
        refresh_db();
        create_folder_db_entry("all tags", None); // 1
        create_folder_db_entry("some tags", Some(1)); // 2
        create_folder_db_entry("no tags", None); // 3
        create_folder_db_entry("no relevant tags", None); // 4
                                                          // tags on them folders
        create_tag_folders("irrelevant", vec![2, 4]);
        create_tag_folders("relevant 1", vec![1, 2]);
        create_tag_folders("relevant 2", vec![1]);
        let con: Connection = open_connection();

        let res = get_folders_by_any_tag(
            &HashSet::from(["relevant 1".to_string(), "relevant 2".to_string()]),
            &con,
        )
        .unwrap()
        .into_iter()
        .collect::<Vec<Folder>>();
        con.close().unwrap();
        assert_eq!(2, res.len());
        assert!(res.contains(&Folder {
            id: Some(1),
            parent_id: None,
            name: "all tags".to_string(),
        }));
        assert!(res.contains(&Folder {
            id: Some(2),
            parent_id: Some(1),
            name: "some tags".to_string(),
        }));
        cleanup();
    }
}

#[cfg(test)]
mod get_parent_folders_by_tag_tests {
    use std::collections::HashSet;

    use crate::repository::folder_repository::get_parent_folders_by_tag;
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_folder_db_entry, create_tag_folder, refresh_db};

    #[test]
    fn retrieves_parent_folders() {
        refresh_db();
        create_folder_db_entry("top", None);
        create_folder_db_entry("middle", Some(1));
        create_folder_db_entry("bottom", Some(2));
        create_tag_folder("tag", 1);
        let con = open_connection();
        let res = get_parent_folders_by_tag(3, &[&"tag".to_string()], &con).unwrap();
        con.close().unwrap();
        assert_eq!(HashSet::from(["tag".to_string()]), *res.get(&1).unwrap());
        cleanup();
    }
}

#[cfg(test)]
mod get_child_files_tests {
    use std::collections::HashSet;

    use crate::repository::folder_repository::get_child_files;
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_file_db_entry, create_folder_db_entry, refresh_db};

    #[test]
    fn get_child_files_works_for_root() {
        refresh_db();
        create_file_db_entry("test", None);
        create_file_db_entry("test2", None);
        create_folder_db_entry("top", None);
        create_file_db_entry("bad", Some(1));
        let con = open_connection();
        let res: HashSet<String> = get_child_files([], &con)
            .unwrap()
            .into_iter()
            .map(|f| f.name)
            .collect();
        con.close().unwrap();
        assert_eq!(
            HashSet::from(["test".to_string(), "test2".to_string()]),
            res
        );
        cleanup();
    }

    #[test]
    fn get_child_files_works_for_non_root() {
        refresh_db();
        create_file_db_entry("bad", None);
        create_folder_db_entry("top", None);
        create_folder_db_entry("middle", Some(1));
        create_file_db_entry("good", Some(1));
        create_file_db_entry("good2", Some(2));
        let con = open_connection();
        let res: HashSet<String> = get_child_files([1, 2], &con)
            .unwrap()
            .into_iter()
            .map(|f| f.name)
            .collect();
        con.close().unwrap();
        assert_eq!(
            HashSet::from(["good".to_string(), "good2".to_string()]),
            res
        );
        cleanup();
    }
}
