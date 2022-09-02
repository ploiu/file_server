#[derive(PartialEq, Debug)]
pub enum GetFolderError {
    NotFound,
    DbFailure,
}

#[derive(PartialEq, Debug)]
pub enum CreateFolderError {
    /// a folder with the name in the selected path already exists
    AlreadyExists,
    /// the database failed to save the folder
    DbFailure,
    /// the file system failed to write the folder
    FileSystemFailure,
    /// the requested parent folder does not exist
    ParentNotFound,
}

#[derive(PartialEq, Debug)]
pub enum UpdateFolderError {
    /// a folder with the name in the selected path already exists
    AlreadyExists,
    /// the database failed to update the folder
    DbFailure,
    /// the file system failed to move the folder
    FileSystemFailure,
    /// the requested parent folder does not exist
    ParentNotFound,
    /// The folder could not be found
    NotFound,
    /// The user attempted to do an illegal action, such as moving a parent folder into its own child
    NotAllowed,
}

#[derive(PartialEq, Debug)]
pub enum GetChildFilesError {
    /// database could not execute the query
    DbFailure,
    /// the folder id could not be found
    FolderNotFound,
}

#[derive(PartialEq, Debug)]
pub enum DeleteFolderError {
    /// database could not execute the query
    DbFailure,
    /// folder not in the repository
    FolderNotFound,
    /// could not remove the folder from the database
    FileSystemError,
}

#[derive(PartialEq, Debug)]
pub enum LinkFolderError {
    DbError,
}
