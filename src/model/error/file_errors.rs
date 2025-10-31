use std::fmt::{Display, Formatter};

#[derive(PartialEq, Debug)]
pub enum CreateFileError {
    FailWriteDisk,
    FailWriteDb,
    ParentFolderNotFound,
    AlreadyExists,
}

#[derive(PartialEq, Debug)]
pub enum GetFileError {
    NotFound,
    DbFailure,
    /// failed to retrieve tags for file
    TagError,
}

#[derive(PartialEq, Debug)]
pub enum GetPreviewError {
    /// a generic error reading from the file system
    FileSystemError,
    /// no preview found for the requested file
    NotFound,
}

/// represents error states that can arise from retrieving all previews for files in a folder
#[derive(PartialEq, Debug)]
pub enum GetBulkPreviewError {
    /// error retrieving files from the database
    Db,
}

#[derive(PartialEq, Debug)]
pub enum DeleteFileError {
    // file reference not found in repository
    NotFound,
    // couldn't remove the file reference from the repository
    DbError,
    // couldn't remove the file from the disk
    FileSystemError,
}

#[derive(PartialEq, Debug)]
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
    /// folder with the new file name already exists in the target directory
    FolderAlreadyExistsWithSameName,
    /// an issue occurred updating or retrieving tags
    TagError,
}

#[derive(PartialEq, Debug)]
pub enum SearchFileError {
    DbError,
    /// an issue occurred retrieving tags
    TagError,
}

impl Display for SearchFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DbError => write!(f, "SearchFileError::DbError"),
            Self::TagError => write!(f, "SearchFileError::TagError"),
        }
    }
}
