#[derive(Debug)]
pub struct FileRecord {
    /// the id, will only be populated when pulled from the database
    pub id: Option<u32>,
    /// the name of the file to save in the db and disk
    pub name: String,
    /// available on retrieve from db
    #[deprecated(note = "this is being removed in favor of new db structure")]
    pub path: Option<String>,
    /// sha256, just to check for uniqueness
    pub hash: String,
}

#[derive(Debug)]
pub struct Folder {
    /// cannot be changed, and only retrieved from the database
    pub id: Option<u32>,
    /// the name of the folder in the db and on the disk
    pub name: String,
    /// may be `None` to represent it being a top-level folder
    pub parent_id: Option<u32>,
}

#[derive(Debug)]
pub struct FolderFiles {
    /// will be None unless this is pulled from the db
    pub id: Option<u32>,
    /// the id of the folder containing the files
    pub folder_id: u32,
    /// the id of the file in this record
    pub file_id: u32,
}

impl FileRecord {
    pub fn from(name: String, hash: String) -> FileRecord {
        FileRecord {
            id: None,
            name,
            path: None,
            hash,
        }
    }
}
