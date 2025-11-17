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

/// represents a tag in the Tags table of the database. When referencing a tag _on_ a file / folder, use [`TaggedItem`] instead
#[derive(Debug, PartialEq, Clone)]
pub struct Tag {
    /// the id of the tag
    pub id: u32,
    /// the display name of the tag
    pub title: String,
}

/// represents a tag on a file or a folder, with optional implication.
/// These are not meant to ever be created outside of a database query retrieving it from the database
///
/// [`file_id`] _or_ [`folder_id`] will be [`None`], but never both. [`implicit_from_id`] will be None if the tag is explicitly on the item
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TaggedItem {
    /// the database id of this specific entry
    pub id: u32,
    /// if present, the id of the file this tag exists on. mutually exclusive with folder_id
    pub file_id: Option<u32>,
    /// if present, the id of the folder this tag exists on. mutually exclusive with file_id
    pub folder_id: Option<u32>,
    /// if present, the folder that implicates this tag on the file/folder this tag applies to
    pub implicit_from_id: Option<u32>,
    /// the tag's title
    pub title: String,
    /// the id of the actual tag
    pub tag_id: u32,
}

impl From<&FileApi> for FileRecord {
    fn from(value: &FileApi) -> Self {
        let create_date = value
            .date_created
            .unwrap_or(chrono::offset::Local::now().naive_local());
        Self {
            id: if value.id == 0 { None } else { Some(value.id) },
            name: value.name.clone(),
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
