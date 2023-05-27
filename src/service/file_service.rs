use std::fs;
use std::fs::File;
use std::path::Path;

use regex::Regex;
use rocket::tokio::fs::create_dir;
use rusqlite::Connection;

use crate::model::error::file_errors::{
    CreateFileError, DeleteFileError, GetFileError, SearchFileError, UpdateFileError,
};
use crate::model::error::folder_errors::{GetFolderError, LinkFolderError};
use crate::model::repository::FileRecord;
use crate::model::request::file_requests::{CreateFileRequest, UpdateFileRequest};
use crate::model::response::file_responses::FileMetadataResponse;
use crate::model::response::folder_responses::FolderResponse;
use crate::repository;
use crate::repository::{file_repository, folder_repository};
use crate::service::folder_service;

pub static FILE_DIR: &str = "./files";

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
) -> Result<FileMetadataResponse, CreateFileError> {
    let file_name = String::from(file_input.file.name().unwrap());
    check_file_in_dir(file_input, &file_name)?;
    check_root_dir(FILE_DIR).await;
    // we shouldn't leak implementation details to the client, so this strips the root dir from the response
    let root_regex = Regex::new(format!("^{}/", FILE_DIR).as_str()).unwrap();
    let parent_id = file_input.folder_id.unwrap_or(0);
    return if parent_id != 0 {
        // we requested a folder to put the file in, so make sure it exists
        let folder = folder_service::get_folder(Some(parent_id))
            .await
            .or_else(|e| {
                eprintln!(
                    "Save file - failed to retrieve parent folder. Nested exception is {:?}",
                    e
                );
                return if e == GetFolderError::NotFound {
                    Err(CreateFileError::ParentFolderNotFound)
                } else {
                    Err(CreateFileError::FailWriteDb)
                };
            })?;
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

/// retrieves the file from the database with the passed id
pub fn get_file_metadata(id: u32) -> Result<FileRecord, GetFileError> {
    let con = repository::open_connection();
    let result = file_repository::get_file(id, &con).or_else(|e| {
        eprintln!(
            "Failed to pull file info from database. Nested exception is {:?}",
            e
        );
        return if e == rusqlite::Error::QueryReturnedNoRows {
            Err(GetFileError::NotFound)
        } else {
            Err(GetFileError::DbFailure)
        };
    });
    con.close().unwrap();
    result
}

/// reads the contents of the file with the passed id from the disk and returns it
pub fn get_file_contents(id: u32) -> Result<File, GetFileError> {
    let res = get_file_path(id);
    return if let Ok(path) = res {
        let path = format!("{}/{}", FILE_DIR, path);
        File::open(path).or_else(|_| Err(GetFileError::NotFound))
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
    // now that we've determined the file exists, we can remove from the repository
    let con = repository::open_connection();
    let delete_result = delete_file_by_id_with_connection(id, &con);
    con.close().unwrap();
    // helps avoid nested matches
    delete_result?;
    return fs::remove_file(&file_path).or_else(|e| {
        eprintln!(
            "Failed to delete file from disk at location {:?}!\n Nested exception is {:?}",
            file_path, e
        );
        Err(DeleteFileError::FileSystemError)
    });
}

/// uses an existing connection to delete file. Exists as an optimization to avoid having to open tons of repository connections when deleting a folder
pub fn delete_file_by_id_with_connection(
    id: u32,
    con: &Connection,
) -> Result<FileRecord, DeleteFileError> {
    let result = match file_repository::delete_file(id, &con) {
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

pub async fn update_file(file: UpdateFileRequest) -> Result<FileMetadataResponse, UpdateFileError> {
    // first check if the file exists
    let con = repository::open_connection();
    if file_repository::get_file(file.id, &con).is_err() {
        con.close().unwrap();
        return Err(UpdateFileError::NotFound);
    }
    // now check if the folder exists
    let parent_folder = folder_service::get_folder(file.folder_id)
        .await
        .or_else(|_| Err(UpdateFileError::FolderNotFound))?;
    // now check if a file with the passed name is already under that folder
    let name_regex = Regex::new(format!("{}$", file.name).as_str()).unwrap();
    for f in parent_folder.files.iter() {
        if name_regex.is_match(f.name.as_str()) {
            return Err(UpdateFileError::FileAlreadyExists);
        }
    }
    // we have to create this before we update the file
    let old_path = format!(
        "{}/{}",
        FILE_DIR,
        file_repository::get_file_path(file.id, &con).unwrap()
    );
    // now that we've verified that the file & folder exist and we're not gonna collide names, perform the move
    let new_parent_id = if file.folder_id == Some(0) {
        None
    } else {
        file.folder_id
    };
    if let Err(e) = file_repository::update_file(&file.id, &new_parent_id, &file.name, &con) {
        con.close().unwrap();
        eprintln!(
            "Failed to update file record in database. Nested exception is {:?}",
            e
        );
        return Err(UpdateFileError::DbError);
    }
    // now that we've updated the file in the database, it's time to update the file system
    let new_path = format!("{}/{}/{}", FILE_DIR, parent_folder.path, file.name);
    // we're done with the database for now
    con.close().unwrap();
    let new_path = Regex::new("/root").unwrap().replace(new_path.as_str(), "");
    if let Err(e) = fs::rename(old_path, new_path.to_string()) {
        eprintln!(
            "Failed to move file in the file system. Nested exception is {:?}",
            e
        );
        return Err(UpdateFileError::FileSystemError);
    }
    Ok(FileMetadataResponse {
        id: file.id,
        name: file.name,
    })
}

pub fn search_files(criteria: String) -> Result<Vec<FileMetadataResponse>, SearchFileError> {
    let con = repository::open_connection();
    let files = match file_repository::search_files(criteria, &con) {
        Ok(f) => f,
        Err(e) => {
            con.close().unwrap();
            eprintln!(
                "Failed to retrieve file records from the database. Nested exception is {:?}",
                e
            );
            return Err(SearchFileError::DbError);
        }
    };
    let mut converted_files: Vec<FileMetadataResponse> = Vec::new();
    for file in files.iter() {
        converted_files.push(FileMetadataResponse {
            id: file.id.unwrap(),
            name: String::from(&file.name),
        })
    }
    Ok(converted_files)
}

// ==== private functions ==== \\

/// persists the file to the disk and the database
async fn persist_save_file_to_folder(
    file_input: &mut CreateFileRequest<'_>,
    folder: &FolderResponse,
    file_name: String,
) -> Result<u32, CreateFileError> {
    let file_name = format!(
        "{}/{}/{}.{}",
        FILE_DIR, folder.path, file_name, file_input.extension
    );
    return match file_input.file.persist_to(&file_name).await {
        Ok(_) => {
            let id = save_file_record(&file_name)?;
            // file and folder are both in repository, now link them
            if link_folder_to_file(id, folder.id).is_err() {
                return Err(CreateFileError::FailWriteDb);
            }
            Ok(id)
        }
        Err(e) => {
            eprintln!("Failed to save file to disk. Nested exception is {:?}", e);
            Err(CreateFileError::FailWriteDisk)
        }
    };
}

/// persists the passed file to the disk and the database
async fn persist_save_file(file_input: &mut CreateFileRequest<'_>) -> Result<u32, CreateFileError> {
    let file_name = format!(
        "{}/{}.{}",
        &FILE_DIR,
        file_input.file.name().unwrap(),
        file_input.extension
    );
    return match file_input.file.persist_to(&file_name).await {
        Ok(_) => Ok(save_file_record(&file_name)?),
        Err(e) => {
            eprintln!("Failed to save file to disk. Nested exception is {:?}", e);
            Err(CreateFileError::FailWriteDisk)
        }
    };
}

fn save_file_record(name: &String) -> Result<u32, CreateFileError> {
    // remove the './' from the file name
    let begin_path_regex = Regex::new("\\.?(/.*/)+?").unwrap();
    let formatted_name = begin_path_regex.replace(&name, "");
    let file_record = FileRecord::from(formatted_name.to_string());
    let con = repository::open_connection();
    let res = file_repository::create_file(&file_record, &con)
        .or_else(|_| Err(CreateFileError::FailWriteDb));
    con.close().unwrap();
    res
}

/// retrieves the full path to the file with the passed id
fn get_file_path(id: u32) -> Result<String, GetFileError> {
    let con = repository::open_connection();
    let result = file_repository::get_file_path(id, &con).or_else(|e| {
        eprintln!("Failed to get file path! Nested exception is {:?}", e);
        return if e == rusqlite::Error::QueryReturnedNoRows {
            Err(GetFileError::NotFound)
        } else {
            Err(GetFileError::DbFailure)
        };
    });
    con.close().unwrap();
    result
}

/// adds a link to the folder for the passed file in the database
fn link_folder_to_file(file_id: u32, folder_id: u32) -> Result<(), LinkFolderError> {
    let con = repository::open_connection();
    let link_result = folder_repository::link_folder_to_file(file_id, folder_id, &con);
    con.close().unwrap();
    if link_result.is_err() {
        return Err(LinkFolderError::DbError);
    }
    return Ok(());
}

/// checks the db to see if we have a record of the passed file
fn check_file_in_dir(
    file_input: &mut CreateFileRequest,
    file_name: &String,
) -> Result<(), CreateFileError> {
    let full_file_name = String::from(format!("{}.{}", &file_name, &file_input.extension));
    // first check that the db does not have a record of the file in its directory
    let con = repository::open_connection();
    let child_files = folder_repository::get_files_for_folder(file_input.folder_id, &con);
    con.close().unwrap();
    if child_files.is_err() {
        return Err(CreateFileError::FailWriteDb);
    }
    // compare the names of all the child files
    for child in child_files.unwrap().iter() {
        if child.name.to_lowercase() == full_file_name.to_lowercase() {
            return Err(CreateFileError::AlreadyExists);
        }
    }
    Ok(())
}
