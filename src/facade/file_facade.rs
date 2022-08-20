use crate::db::{file, open_connection};
use crate::model::db::FileRecord;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::Path;

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
