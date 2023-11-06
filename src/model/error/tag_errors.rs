#[derive(PartialEq)]
pub enum CreateTagError {
    /// an error with the database
    DbError,
}

#[derive(PartialEq)]
pub enum GetTagError {
    /// an error with the database
    DbError,
    /// the tag was not found
    TagNotFound,
}

#[derive(PartialEq)]
pub enum UpdateTagError {
    /// an error with the database
    DbError,
    /// no tag with that id can be found
    TagNotFound,
    /// a tag with the selected name already exists, and is not the tag being updated
    NewNameAlreadyExists,
}

#[derive(PartialEq)]
pub enum DeleteTagError {
    /// an error with the database
    DbError,
}

#[derive(PartialEq)]
pub enum TagRelationError {
    /// an error with the database
    DbError,
    /// no file with the passed id was found
    FileNotFound,
    /// no folder with the passed id was found
    FolderNotFound,
    /// no tag with the passed id was found
    TagNotFound,
}
