use core::option::Option;
use rocket::fs::TempFile;

#[derive(FromForm)]
pub struct CreateFileRequest<'a> {
    /// the file being uploaded
    pub file: TempFile<'a>,
    /// because I don't feel like mapping from content-type header
    pub extension: &'a str,
    /// leave blank for top level folder
    pub folderId: Option<u32>,
}
