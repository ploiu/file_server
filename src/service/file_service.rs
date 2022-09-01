use regex::Regex;
use std::fs::File;
use std::path::Path;

use crate::db;
use crate::db::{file_repository, folder_repository};
use rocket::tokio::fs::create_dir;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::model::db::FileRecord;
use crate::model::request::file_requests::CreateFileRequest;
use crate::model::response::file_responses::FileMetadataResponse;
use crate::model::response::folder_responses::FolderResponse;
use crate::service::folder_service;
use crate::service::folder_service::{GetFolderError, LinkFolderError};

pub static FILE_DIR: &str = "./files";

#[derive(PartialEq)]
pub enum SaveFileError {
    #[allow(dead_code)] // this is actually used. Thanks rust linter!
    MissingInfo(String),
    FailWriteDisk,
    FailWriteDb,
    ParentFolderNotFound,
}

#[derive(PartialEq)]
pub enum GetFileError {
    NotFound,
    DbFailure,
}

#[derive(PartialEq)]
pub enum DeleteFileError {
    // file reference not found in db
    NotFound,
    // couldn't remove the file reference from the db
    DbError,
    // couldn't remove the file from the disk
    FileSystemError,
}

/// ensures that the passed directory exists on the file system
pub async fn check_root_dir(dir: &str) {
    let path = Path::new(dir);
    if !path.exists() {
        if let Err(e) = create_dir(path).await {
            panic!("Failed to create file directory: \n {:?}", e)
        }
    }
}

/// saves a file to the disk and database
pub async fn save_file(
    file_input: &mut CreateFileRequest<'_>,
) -> Result<FileMetadataResponse, SaveFileError> {
    let file_name = String::from(file_input.file.name().unwrap());
    check_root_dir(FILE_DIR).await;
    // we shouldn't leak implementation details to the client, so this strips the root dir from the response
    let root_regex = Regex::new(format!("^{}/", FILE_DIR).as_str()).unwrap();
    return if let Some(parent_id) = file_input.folder_id {
        // we requested a folder to put the file in, so make sure it exists
        let folder = match folder_service::get_folder(Some(parent_id)) {
            Ok(f) => f,
            Err(e) if e == GetFolderError::NotFound => {
                return Err(SaveFileError::ParentFolderNotFound)
            }
            Err(e) => {
                eprintln!(
                    "Save file - failed to retrieve parent folder. Nested exception is: \n {:?}",
                    e
                );
                return Err(SaveFileError::FailWriteDb);
            }
        };
        // folder exists, now try to create the file
        let file_id =
            persist_save_file_to_folder(file_input, &folder, String::from(&file_name)).await?;
        Ok(FileMetadataResponse {
            id: file_id,
            name: String::from(root_regex.replace(&file_name, "")),
        })
    } else {
        let file_name = format!("{}/{}.{}", &FILE_DIR, file_name, file_input.extension);
        let file_id = persist_save_file(file_input).await?;
        Ok(FileMetadataResponse {
            id: file_id,
            name: String::from(root_regex.replace(&file_name, "")),
        })
    };
}

pub fn get_file(id: u32) -> Result<FileRecord, GetFileError> {
    let con = db::open_connection();
    let result = match file_repository::get_by_id(id, &con) {
        Ok(record) => Ok(record),
        Err(error) if error == rusqlite::Error::QueryReturnedNoRows => Err(GetFileError::NotFound),
        Err(error) => {
            eprintln!(
                "Failed to pull file info from database! Nested exception is: \n {:?}",
                error
            );
            Err(GetFileError::DbFailure)
        }
    };
    con.close().unwrap();
    result
}

pub fn download_file(id: u32) -> Result<File, GetFileError> {
    let res = get_file_path(id);
    return if let Ok(path) = res {
        let path = format!("{}/{}", FILE_DIR, path);
        match File::open(path) {
            Ok(f) => Ok(f),
            Err(_) => Err(GetFileError::NotFound),
        }
    } else {
        Err(res.unwrap_err())
    };
}

pub fn delete_file(id: u32) -> Result<(), DeleteFileError> {
    let file_path = match get_file_path(id) {
        Ok(path) => format!("{}/{}", FILE_DIR, path),
        Err(e) if e == GetFileError::NotFound => return Err(DeleteFileError::NotFound),
        Err(_) => return Err(DeleteFileError::DbError),
    };
    // now that we've determined the file exists, we can remove from the db
    let con = db::open_connection();
    let delete_result = delete_file_by_id_with_connection(id, &con);
    con.close().unwrap();
    // helps avoid nested matches
    delete_result?;
    return std::fs::remove_file(&file_path).or_else(|e| {
        eprintln!(
            "Failed to delete file from disk at location {:?}!\n Nested exception is {:?}",
            file_path, e
        );
        Err(DeleteFileError::FileSystemError)
    });
}

/// uses an existing connection to delete file. Exists as an optimization to avoid having to open tons of db connections when deleting a folder
pub fn delete_file_by_id_with_connection(
    id: u32,
    con: &Connection,
) -> Result<FileRecord, DeleteFileError> {
    let result = match file_repository::delete_by_id(id, &con) {
        Ok(record) => Ok(record),
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Err(DeleteFileError::NotFound),
        Err(e) => {
            eprintln!(
                "Failed to delete file record from database! Nested exception is: \n {:?}",
                e
            );
            Err(DeleteFileError::DbError)
        }
    };
    return result;
}

// ==== private functions ==== \\

/// persists the file to the disk and the database
async fn persist_save_file_to_folder(
    file_input: &mut CreateFileRequest<'_>,
    folder: &FolderResponse,
    file_name: String,
) -> Result<u32, SaveFileError> {
    let file_name = format!(
        "{}/{}/{}.{}",
        FILE_DIR, folder.path, file_name, file_input.extension
    );
    return match file_input.file.persist_to(&file_name).await {
        Ok(_) => {
            // since this is guaranteed to happen after the file is successfully saved, we can unwrap here
            let mut saved_file = File::open(&file_name).unwrap();
            match save_file_record(&file_name, &mut saved_file) {
                Ok(id) => {
                    // file and folder are both in db, now link them
                    if link_folder_to_file(id, folder.id).is_err() {
                        return Err(SaveFileError::FailWriteDb);
                    }
                    Ok(id)
                }
                Err(e) => {
                    eprintln!("Failed to create file record in database: {:?}", e);
                    Err(SaveFileError::FailWriteDb)
                }
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            Err(SaveFileError::FailWriteDisk)
        }
    };
}

/// persists the passed file to the disk and the database
async fn persist_save_file(file_input: &mut CreateFileRequest<'_>) -> Result<u32, SaveFileError> {
    let file_name = format!(
        "{}/{}.{}",
        &FILE_DIR,
        file_input.file.name().unwrap(),
        file_input.extension
    );
    return match file_input.file.persist_to(&file_name).await {
        Ok(()) => {
            // since this is guaranteed to happen after the file is successfully saved, we can unwrap here
            let mut saved_file = File::open(&file_name).unwrap();
            match save_file_record(&file_name, &mut saved_file) {
                Err(e) => {
                    eprintln!("Failed to create file record in database: {:?}", e);
                    Err(SaveFileError::FailWriteDb)
                }
                Ok(id) => Ok(id),
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            Err(SaveFileError::FailWriteDisk)
        }
    };
}

fn save_file_record(name: &String, mut file: &mut File) -> Result<u32, String> {
    // hash the file TODO remove - we won't check uniqueness by file hash anymore
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let hash = hasher.finalize();
    // remove the './' from the file name
    let begin_path_regex = Regex::new("\\.?(/.*/)+?").unwrap();
    let mut formatted_name = begin_path_regex.replace(&name, "");
    let hash = format!("{:x}", hash);
    let file_record = FileRecord::from(formatted_name.to_mut().to_string(), hash);
    let con = db::open_connection();
    let res = file_repository::save_file_record(&file_record, &con);
    con.close().unwrap();
    res
}

/// retrieves the full path to the file with the passed id
fn get_file_path(id: u32) -> Result<String, GetFileError> {
    let con = db::open_connection();
    let result = match file_repository::get_file_path(id, &con) {
        Ok(path) => Ok(path),
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Err(GetFileError::NotFound),
        Err(e) => {
            eprintln!("Failed to get file path! Nested exception is: \n {:?}", e);
            Err(GetFileError::DbFailure)
        }
    };
    con.close().unwrap();
    result
}

/// adds a link to the folder for the passed file in the database
fn link_folder_to_file(file_id: u32, folder_id: u32) -> Result<(), LinkFolderError> {
    let con = db::open_connection();
    let link_result = folder_repository::link_folder_to_file(file_id, folder_id, &con);
    con.close().unwrap();
    if link_result.is_err() {
        return Err(LinkFolderError::DbError);
    }
    return Ok(());
}
