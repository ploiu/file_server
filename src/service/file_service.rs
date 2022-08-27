use regex::Regex;
use std::fs::File;
use std::path::Path;

use rocket::tokio::fs::create_dir;

use crate::facade::file_facade::{delete_file_by_id, get_file_info_by_id, save_file_record};
use crate::facade::{file_facade, folder_facade};
use crate::model::db::{FileRecord, Folder};
use crate::model::request::file_requests::CreateFileRequest;
use crate::model::response::file_responses::FileMetadataResponse;
use crate::service::folder_service::GetFolderError;

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
        match create_dir(path).await {
            Ok(_) => (),
            Err(e) => panic!("Failed to create file directory: \n {:?}", e),
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
        let folder = match folder_facade::get_folder_by_id(Some(parent_id)) {
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

/// persists the file to the disk and the database
async fn persist_save_file_to_folder(
    file_input: &mut CreateFileRequest<'_>,
    folder: &Folder,
    file_name: String,
) -> Result<u32, SaveFileError> {
    let file_name = format!(
        "{}/{}/{}.{}",
        FILE_DIR, folder.name, file_name, file_input.extension
    );
    return match file_input.file.persist_to(&file_name).await {
        Ok(_) => {
            // since this is guaranteed to happen after the file is successfully saved, we can unwrap here
            let mut saved_file = File::open(&file_name).unwrap();
            match file_facade::save_file_record(&file_name, &mut saved_file) {
                Ok(id) => {
                    // file and folder are both in db, now link them
                    if let Err(_) = folder_facade::link_folder_to_file(id, folder.id.unwrap()) {
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

pub fn get_file(id: u32) -> Result<FileRecord, GetFileError> {
    get_file_info_by_id(id)
}

pub fn download_file(id: u32) -> Result<File, GetFileError> {
    let res = file_facade::get_file_path(id);
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
    let file_path = match file_facade::get_file_path(id) {
        Ok(path) => format!("{}/{}", FILE_DIR, path),
        Err(e) if e == GetFileError::NotFound => return Err(DeleteFileError::NotFound),
        Err(_) => return Err(DeleteFileError::DbError),
    };
    match delete_file_by_id(id) {
        Ok(_) => {
            match std::fs::remove_file(&file_path) {
                Ok(()) => Ok(()),
                Err(e) => {
                    eprintln!("Failed to delete file from disk at location {:?}!\n Nested exception is {:?}", file_path, e);
                    Err(DeleteFileError::FileSystemError)
                }
            }
        }
        Err(e) => Err(e),
    }
}
