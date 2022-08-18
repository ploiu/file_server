#[derive(Debug)]
pub struct FileRecord<'fr> {
    id: u64,
    name: &'fr str,
    path: &'fr str,
    // md5, just to check for uniqueness
    hash: &'fr str,
}
