use chrono::NaiveDateTime;
use rocket::serde::Serialize;

use super::api::FileApi;
use super::file_types::FileTypes;

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

impl From<&FileApi> for FileRecord {
    fn from(value: &FileApi) -> Self {
        let create_date = value
            .date_created
            .unwrap_or(chrono::offset::Local::now().naive_local());
        Self {
            id: if value.id == 0 { None } else { Some(value.id) },
            name: value.name().unwrap_or_default().clone(),
            parent_id: if value.folder_id == Some(0) {
                None
            } else {
                value.folder_id
            },
            create_date,
            // will be 0 if a size needs to be set
            size: value.size.unwrap_or_default(),
            file_type: value.file_type.unwrap_or(FileTypes::Unknown),
        }
    }
}
