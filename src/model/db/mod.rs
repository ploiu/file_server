#[derive(Debug)]
pub struct FileRecord {
    pub id: Option<u32>,
    pub name: String,
    /// available on retrieve from db
    pub path: Option<String>,
    /// sha256, just to check for uniqueness
    pub hash: String,
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
