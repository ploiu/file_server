use std::fs::{remove_dir_all, remove_file};
use std::path::Path;

use crate::repository::initialize_db;
use crate::service::file_service::FILE_DIR;

/// username:password
#[cfg(test)]
pub static AUTH: &str = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";

#[cfg(test)]
pub fn refresh_db() {
    remove_file(Path::new("db.sqlite")).unwrap();
    initialize_db().unwrap();
}

#[cfg(test)]
pub fn remove_files() {
    if Path::new(FILE_DIR).exists() {
        remove_dir_all(Path::new("files")).unwrap();
    }
}
