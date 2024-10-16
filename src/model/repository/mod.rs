use chrono::NaiveDateTime;
use rocket::serde::Serialize;

use super::api::FileTypes;

#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Debug, Serialize, Eq, Hash, Clone)]
// for testing we have to ignore the create_date field when doing equality checking otherwise it's an inconsistent pita
#[cfg_attr(not(test), derive(PartialEq))]
#[serde(crate = "rocket::serde")]
pub struct FileRecord {
    /// the id, will only be populated when pulled from the database
    pub id: Option<u32>,
    /// the name of the file to save in the repository and disk
    pub name: String,
    /// will be None if in the root folder
    pub parent_id: Option<u32>,
    /// the date the file was uploaded to the server
    pub create_date: NaiveDateTime,
    pub size: u64,
    pub file_type: FileTypes,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Folder {
    /// cannot be changed, and only retrieved from the database
    pub id: Option<u32>,
    /// the name of the folder in the repository and on the disk
    pub name: String,
    /// may be `None` to represent it being a top-level folder
    pub parent_id: Option<u32>,
}

#[derive(Debug)]
pub struct FolderFiles {
    /// will be None unless this is pulled from the repository
    pub id: Option<u32>,
    /// the id of the folder containing the files
    pub folder_id: u32,
    /// the id of the file in this record
    pub file_id: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tag {
    /// the id of the tag
    pub id: u32,
    /// the display name of the tag
    pub title: String,
}

#[derive(Debug)]
pub struct FilePreview {
    /// the id of the file this preview is for
    pub id: u32,
    /// the binary contents of the file preview.
    /// This is stored in jpeg format
    pub file_preview: Vec<u8>,
}
