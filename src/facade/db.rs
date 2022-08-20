use std::fs::File;
use std::path::Path;

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::db::{file, metadata};
use crate::db::metadata::{CheckAuthResult, get_auth, set_auth};
use crate::db::open_connection;
use crate::guard::Auth;
use crate::model::db::FileRecord;

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
        formatted_name.to_mut(),
        path.to_str().unwrap(),
        hash.as_str(),
    );
    let res = file::save_file_record(&file_record, &con);
    con.close().unwrap();
    res
}

/// Checks if the passed `auth` object matches the password in the database
pub fn check_auth(auth: Auth) -> CheckAuthResult {
    let mut con = open_connection();
    let result = metadata::check_auth(auth, &mut con);
    con.close().unwrap();
    result
}

/// checks if a password was set in the database
pub fn is_password_set() -> bool {
    let mut con = open_connection();
    let has_password = match get_auth(&mut con) {
        Ok(_) => true,
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => false,
        Err(e) => {
            panic!("Failed to check auth in database: {:?}", e);
        }
    };
    con.close().unwrap();
    has_password
}

/// saves the passed auth to the database.
///
/// This should never be called if there is a password already set (see `is_password_set`), because
/// it will override whatever is already in the database.
pub fn set_password(auth: Auth) -> bool {
    let mut con = open_connection();
    let result = set_auth(auth, &mut con);
    con.close().unwrap();
    result
}
