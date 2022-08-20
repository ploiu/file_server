#[derive(Debug)]
pub struct FileRecord {
    pub id: Option<u64>,
    pub name: String,
    pub path: String,
    // md5, just to check for uniqueness
    pub hash: String,
}

impl FileRecord {
    pub fn from(name: String, path: String, hash: String) -> FileRecord {
        FileRecord {
            id: None,
            name,
            path,
            hash,
        }
    }
}
