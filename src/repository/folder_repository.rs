use std::backtrace::Backtrace;
use std::collections::HashSet;

use rusqlite::{Connection, Rows, params};

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
                name: folder.name.clone(),
                parent_id: folder.parent_id,
            })
        }
        None => {
            let folder_id =
                pst.insert(rusqlite::params![folder.name, rusqlite::types::Null])? as u32;
            Ok(repository::Folder {
                id: Some(folder_id),
                name: folder.name.clone(),
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

/// Retrieve files that are children of the given folder IDs.
///
/// This function accepts any iterable collection of u32 folder IDs and returns
/// a Vec of repository::FileRecord for files directly under those folder IDs.
/// If the provided collection of IDs is empty, the function
/// treats that as a request for files in the root folder and will return files
/// directly under the root folder.
///
/// Important: to retrieve files in the root folder pass an empty collection for
/// `ids`. Passing a collection containing 0 (or other sentinel values) is not
/// treated as the root — the collection must be empty for root behavior.
///
/// # Parameters
/// - `ids`: an iterable collection of u32 folder IDs. If empty, root files are returned.
/// - `con`: reference to an active rusqlite::Connection.
///
/// # Returns
/// On success, returns Ok(Vec<repository::FileRecord>) containing the child files.
/// Otherwise returns a rusqlite::Error.
///
/// # Examples
/// ```no_run
/// // get files in root
/// let files = get_child_files([], &con)?;
/// // get files in folders 1 and 2
/// let files = get_child_files([1u32, 2u32], &con)?;
/// ```
pub fn get_child_files(
    ids: &[u32],
    con: &Connection,
) -> Result<Vec<repository::FileRecord>, rusqlite::Error> {
    // `is_empty` is not part of a trait, so we have to convert ids
    let ids: HashSet<u32> = ids.iter().copied().collect();
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
            log::error!(
                "Failed to link file to folder. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            Err(e)
        }
    }
}

/// returns all the ids of all child folders recursively for the passed input_ids
pub fn get_all_child_folder_ids(
    input_ids: &[u32],
    con: &Connection,
) -> Result<Vec<u32>, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/folder/get_child_folder_ids_recursive.sql"
        ))
        .unwrap();
    let input_ids: HashSet<u32> = input_ids.iter().copied().collect();
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

/// Retrieves all ids of the ancestor folders of the folder with the passed `folder_id`.
///
/// Ancestor id order is guaranteed to be in order of closest parent to the folder first.
/// For example, if called in folder D in A/B/C/D/E, it will return [C, B, A]
///
/// ## Parameters:
/// - `folder_id`: the id of the folder whose ancestors need to be retrieved
/// - `con`: a connection to the database. Must be closed by the caller
pub fn get_ancestor_folders_with_id(
    folder_id: u32,
    con: &Connection,
) -> Result<Vec<u32>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/folder/get_ancestor_folders_with_id.sql"
    ))?;
    // while it's possible for a folder to be nested more than 5 layers deep, 5 is a good starting tradeoff for most folders (at least for my use case)
    let mut ids: Vec<u32> = Vec::with_capacity(5);
    let mut retrieved = pst.query([folder_id])?;
    while let Some(id) = retrieved.next()? {
        ids.push(id.get(0)?);
    }
    Ok(ids)
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
mod get_child_files_tests {
    use std::collections::HashSet;

    use crate::repository::folder_repository::get_child_files;
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_file_db_entry, create_folder_db_entry, init_db_folder};

    #[test]
    fn get_child_files_works_for_root() {
        init_db_folder();
        create_file_db_entry("test", None);
        create_file_db_entry("test2", None);
        create_folder_db_entry("top", None);
        create_file_db_entry("bad", Some(1));
        let con = open_connection();
        let res: HashSet<String> = get_child_files(&[], &con)
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
        init_db_folder();
        create_file_db_entry("bad", None);
        create_folder_db_entry("top", None);
        create_folder_db_entry("middle", Some(1));
        create_file_db_entry("good", Some(1));
        create_file_db_entry("good2", Some(2));
        let con = open_connection();
        let res: HashSet<String> = get_child_files(&[1, 2], &con)
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

#[cfg(test)]
mod get_ancestor_folders_with_id {
    use crate::repository::folder_repository::get_ancestor_folders_with_id;
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_folder_db_entry, init_db_folder};

    #[test]
    fn should_return_empty_vec_if_no_parents() {
        init_db_folder();
        create_folder_db_entry("top", None);
        let con = open_connection();
        let res = get_ancestor_folders_with_id(1, &con).unwrap();
        con.close().unwrap();
        assert!(res.is_empty());
        cleanup();
    }

    #[test]
    fn should_return_empty_vec_if_folder_does_not_exist() {
        init_db_folder();
        let con = open_connection();
        let res = get_ancestor_folders_with_id(999, &con).unwrap();
        con.close().unwrap();
        assert!(res.is_empty());
        cleanup();
    }

    #[test]
    fn should_return_ancestors_in_depth_first_order() {
        init_db_folder();
        create_folder_db_entry("A", None);
        create_folder_db_entry("B", Some(1));
        create_folder_db_entry("C", Some(2));
        create_folder_db_entry("D", Some(3));
        let con = open_connection();
        let res = get_ancestor_folders_with_id(4, &con).unwrap();
        con.close().unwrap();
        assert_eq!(vec![3, 2, 1], res);
        cleanup();
    }
}
