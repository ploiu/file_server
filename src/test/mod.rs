use std::fs;
use std::fs::{remove_dir_all, remove_file};
use std::path::Path;

use crate::model::repository::{FileRecord, Folder};
use crate::repository::{
    file_repository, folder_repository, initialize_db, open_connection, tag_repository,
};
use crate::service::file_service::file_dir;
use crate::temp_dir;

/// username:password
#[cfg(test)]
pub static AUTH: &str = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";

#[cfg(test)]
pub fn refresh_db() {
    let thread_name = current_thread_name();
    remove_file(Path::new(format!("{thread_name}.sqlite").as_str())).unwrap_or(());
    initialize_db().unwrap();
}

#[cfg(test)]
pub fn remove_files() {
    let thread_name = current_thread_name();
    let file_path = Path::new(thread_name.as_str());
    if file_path.exists() {
        remove_dir_all(file_path).unwrap_or(());
    }
}

#[cfg(test)]
pub fn create_file_db_entry(name: &str, folder_id: Option<u32>) {
    let connection = open_connection();
    let file_id = file_repository::create_file(
        &FileRecord {
            id: folder_id,
            name: String::from(name),
            parent_id: None,
        },
        &connection,
    )
    .unwrap();
    if let Some(id) = folder_id {
        folder_repository::link_folder_to_file(file_id, id, &connection).unwrap();
    }
    connection.close().unwrap();
}

#[cfg(test)]
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

#[cfg(test)]
pub fn create_tag_db_entry(name: &str) -> u32 {
    let connection = open_connection();
    let id = tag_repository::create_tag(&name.to_string(), &connection)
        .unwrap()
        .id;
    connection.close().unwrap();
    id
}

#[cfg(test)]
pub fn create_tag_folder(name: &str, folder_id: u32) {
    let connection = open_connection();
    let id = create_tag_db_entry(name);
    tag_repository::add_tag_to_folder(folder_id, id, &connection).unwrap();
    connection.close().unwrap();
}

#[cfg(test)]
pub fn create_tag_folders(name: &str, folder_ids: Vec<u32>) {
    let connection = open_connection();
    let id = create_tag_db_entry(name);
    for folder_id in folder_ids {
        tag_repository::add_tag_to_folder(folder_id, id, &connection).unwrap();
    }
    connection.close().unwrap();
}

#[cfg(test)]
pub fn create_tag_file(name: &str, file_id: u32) {
    let connection = open_connection();
    let id = create_tag_db_entry(name);
    tag_repository::add_tag_to_file(file_id, id, &connection).unwrap();
    connection.close().unwrap();
}

#[cfg(test)]
pub fn create_tag_files(name: &str, file_ids: Vec<u32>) {
    let connection = open_connection();
    let id = create_tag_db_entry(name);
    for file_id in file_ids {
        tag_repository::add_tag_to_file(file_id, id, &connection).unwrap();
    }
    connection.close().unwrap();
}

#[cfg(test)]
pub fn fail() {
    panic!("unimplemented test");
}

#[cfg(test)]
pub fn current_thread_name() -> String {
    let current_thread = std::thread::current();
    current_thread.name().unwrap().to_string()
}

#[cfg(test)]
pub fn create_file_disk(file_name: &str, contents: &str) {
    fs::create_dir(Path::new(file_dir().as_str())).unwrap_or(());
    fs::write(
        Path::new(format!("{}/{file_name}", file_dir()).as_str()),
        contents,
    )
    .unwrap();
}

#[cfg(test)]
pub fn create_folder_disk(folder_name: &str) {
    fs::create_dir_all(Path::new(format!("{}/{folder_name}", file_dir()).as_str())).unwrap();
}

#[cfg(test)]
pub fn cleanup() {
    let thread_name = current_thread_name();
    let temp_dir_name = temp_dir();
    remove_files();
    remove_file(Path::new(format!("{thread_name}.sqlite").as_str())).unwrap_or(());
    remove_dir_all(Path::new(temp_dir_name.as_str())).unwrap_or(());
}
