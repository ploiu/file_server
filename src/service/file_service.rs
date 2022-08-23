use std::fs::File;
use std::path::Path;

use rocket::tokio::fs::create_dir;

use crate::facade::file_facade::{delete_file_by_id, get_file_info_by_id, save_file_record};
use crate::model::request::file::CreateFileRequest;

static FILE_DIR: &str = "./files";

#[derive(PartialEq)]
pub enum SaveFileError {
    MissingInfo(String),
    FailWriteDisk,
    FailWriteDb,
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
async fn check_image_dir(dir: &str) {
    let path = Path::new(dir);
    if !path.exists() {
        match create_dir(path).await {
            Ok(_) => (),
            Err(e) => panic!("Failed to create file directory: \n {:?}", e),
        }
    }
}

/// saves a file to the disk and database
pub async fn save_file(file_input: &mut CreateFileRequest<'_>) -> Result<(), SaveFileError> {
    check_image_dir(FILE_DIR).await;
    let file_name = match file_input.file.name() {
        Some(name) => name,
        None => {
            return Err(SaveFileError::MissingInfo(
                "file name is required".to_string(),
            ))
        }
    };
    // create the file name from the parts
    let file_name = format!("{}/{}.{}", &FILE_DIR, file_name, file_input.extension);
    let path = Path::new(file_name.as_str());
    match file_input.file.persist_to(path).await {
        Ok(_) => {
            // since this is guaranteed to happen after the file is successfully saved, we can unwrap here
            let mut saved_file = File::open(path).unwrap();
            match save_file_record(&file_name, &path, &mut saved_file) {
                Err(e) => {
                    eprintln!("Failed to create file record in database: {:?}", e);
                    return Err(SaveFileError::FailWriteDb);
                }
                _ => {}
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(SaveFileError::FailWriteDisk);
        }
    }
    return Ok(());
}

pub fn get_file(id: u64) -> Result<File, GetFileError> {
    match get_file_info_by_id(id) {
        Ok(file_info) => {
            // TODO the file may not exist on the disk
            let built_path = format!("./files/{}/{}", file_info.path.unwrap(), file_info.name);
            return Ok(File::open(Path::new(built_path.as_str())).unwrap());
        }
        Err(e) => Err(e),
    }
}

pub fn delete_file(id: u64) -> Result<(), DeleteFileError> {
    match delete_file_by_id(id) {
        Ok(file_record) => {
            let file_path = Path::new("FIXME");
            match std::fs::remove_file(file_path) {
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
