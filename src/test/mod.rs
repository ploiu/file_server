use crate::model::repository::{FileRecord, Folder};
use std::fs::{remove_dir_all, remove_file};
use std::path::Path;

use crate::repository::{file_repository, folder_repository, initialize_db, open_connection};
use crate::service::file_service::FILE_DIR;

/// username:password
#[cfg(test)]
pub static AUTH: &str = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";

#[cfg(test)]
pub fn refresh_db() {
    remove_file(Path::new("db.sqlite"))
        .or(Ok::<(), ()>(()))
        .unwrap();
    initialize_db().unwrap();
}

#[cfg(test)]
pub fn remove_files() {
    if Path::new(FILE_DIR).exists() {
        remove_dir_all(Path::new("files"))
            .or(Ok::<(), ()>(()))
            .unwrap();
    }
}

pub fn create_file_db_entry(name: &str, folder_id: Option<u32>) {
    let connection = open_connection();
    let file_id = file_repository::create_file(
        &FileRecord {
            id: folder_id,
            name: String::from(name),
        },
        &connection,
    )
    .unwrap();
    if let Some(id) = folder_id {
        folder_repository::link_folder_to_file(file_id, id, &connection).unwrap();
    }
    connection.close().unwrap();
}

pub fn create_folder_db_entry(name: &str, parent_id: Option<u32>) {
    let connection = open_connection();
    folder_repository::create_folder(
        &Folder {
            id: None,
            name: String::from(name),
            parent_id,
        },
        &connection,
    )
    .unwrap();
    connection.close().unwrap();
}
