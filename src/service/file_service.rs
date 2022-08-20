use std::fs::File;
use std::path::Path;

use rocket::http::Status;
use rocket::tokio::fs::create_dir;

use crate::facade::file_facade::save_file_record;
use crate::model::request::FileUpload;
use crate::model::response::BasicResponse;

static FILE_DIR: &str = "./files";

#[derive(PartialEq)]
pub enum FileError {
    MissingInfo(String),
    FailWriteDisk,
    FailWriteDb,
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
pub async fn save_file<'a>(file_input: &mut FileUpload<'_>) -> Result<(), FileError> {
    check_image_dir(FILE_DIR).await;
    let file_name = match file_input.file.name() {
        Some(name) => name,
        None => return Err(FileError::MissingInfo("file name is required".to_string())),
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
                    return Err(FileError::FailWriteDb);
                }
                _ => {}
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(FileError::FailWriteDisk);
        }
    }
    return Ok(());
}

pub async fn get_file<'a>(id: u64) -> (Status, BasicResponse<'a>) {}
