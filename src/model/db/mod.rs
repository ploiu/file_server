#[derive(Debug)]
pub struct FileRecord<'fr> {
    pub id: u64,
    pub name: &'fr str,
    pub path: &'fr str,
    // md5, just to check for uniqueness
    pub hash: &'fr str,
}
