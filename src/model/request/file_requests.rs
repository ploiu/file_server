use core::option::Option;
use rocket::fs::TempFile;

#[derive(FromForm)]
pub struct CreateFileRequest<'a> {
    /// the file being uploaded
    pub file: TempFile<'a>,
    /// because I don't feel like mapping from content-type header
    pub extension: String,
    /// leave blank for top level folder
    pub folder_id: Option<u32>,
}
