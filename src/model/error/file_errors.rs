#[derive(PartialEq)]
pub enum SaveFileError {
    #[allow(dead_code)] // this is actually used. Thanks rust linter!
    MissingInfo(String),
    FailWriteDisk,
    FailWriteDb,
    ParentFolderNotFound,
}

#[derive(PartialEq)]
pub enum GetFileError {
    NotFound,
    DbFailure,
}

#[derive(PartialEq)]
pub enum DeleteFileError {
    // file reference not found in repository
    NotFound,
    // couldn't remove the file reference from the repository
    DbError,
    // couldn't remove the file from the disk
    FileSystemError,
}
