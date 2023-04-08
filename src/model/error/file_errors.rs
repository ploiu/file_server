#[derive(PartialEq)]
pub enum CreateFileError {
    FailWriteDisk,
    FailWriteDb,
    ParentFolderNotFound,
    AlreadyExists,
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

#[derive(PartialEq)]
pub enum UpdateFileError {
    /// file not found in the db
    NotFound,
    /// Generic database error
    DbError,
    /// Generic filesystem error
    FileSystemError,
    /// requested folder id not found
    FolderNotFound,
    /// file already exists in the target directory
    FileAlreadyExists,
}

#[derive(PartialEq)]
pub enum SearchFileError {
    DbError,
}
