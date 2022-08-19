#[derive(Debug)]
pub struct FileRecord<'fr> {
    pub id: Option<u64>,
    pub name: &'fr str,
    pub path: &'fr str,
    // md5, just to check for uniqueness
    pub hash: &'fr str,
}

impl FileRecord<'_> {
    pub fn from<'a>(name: &'a str, path: &'a str, hash: &'a str) -> FileRecord<'a> {
        FileRecord {
            id: None,
            name,
            path,
            hash,
        }
    }
}
