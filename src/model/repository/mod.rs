use chrono::NaiveDateTime;
use rocket::serde::Serialize;

#[derive(Debug, Serialize, Eq, Hash)]
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
}

#[cfg(not(test))]
impl PartialEq for FileRecord {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.parent_id == other.parent_id
            && self.create_date == other.create_date
            && self.size == other.size
    }
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

// ----------------------------
impl FileRecord {
    pub fn create(name: String, create_date: NaiveDateTime, size: u64) -> FileRecord {
        FileRecord {
            id: None,
            name,
            parent_id: None,
            create_date,
            size,
        }
    }
}
