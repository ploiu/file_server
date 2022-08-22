use std::fs::File;
use std::path::Path;

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::db::file::{delete_by_id, get_by_id};
use crate::db::{file, open_connection};
use crate::model::db::FileRecord;
use crate::service::file_service::{DeleteFileError, GetFileError};

/// saves a record of the passed file info to the database
/// TODO check if file already exists
pub fn save_file_record(name: &str, path: &Path, mut file: &mut File) -> Result<(), String> {
    let begin_path_regex = Regex::new("\\.?(/.*/)+?").unwrap();
    let con = open_connection();
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let hash = hasher.finalize();
    let mut formatted_name = begin_path_regex.replace(&name, "");
    let hash = format!("{:x}", hash);
    let file_record = FileRecord::from(
        formatted_name.to_mut().to_string(),
        path.to_str().unwrap().to_string(),
        hash.as_str().to_string(),
    );
    let res = file::save_file_record(&file_record, &con);
    con.close().unwrap();
    res
}

/// Retrieves a file by the passed id from the database
pub fn get_file_info_by_id(id: u64) -> Result<FileRecord, GetFileError> {
    let mut con = open_connection();
    let result = match get_by_id(id, &con) {
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

pub fn delete_file_by_id(id: u64) -> Result<FileRecord, DeleteFileError> {
    let mut con = open_connection();
    let result = match delete_by_id(id, &con) {
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
    con.close().unwrap();
    return result;
}
