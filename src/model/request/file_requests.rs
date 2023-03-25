use core::option::Option;

use rocket::fs::TempFile;
use rocket::serde::Deserialize;

#[derive(FromForm)]
pub struct CreateFileRequest<'a> {
    /// the file being uploaded
    pub file: TempFile<'a>,
    /// because I don't feel like mapping from content-type header
    pub extension: String,
    /// leave blank for top level folder TODO rename to folderId
    pub folder_id: Option<u32>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateFileRequest {
    pub id: u32,
    pub name: String,
    #[serde(rename = "folderId")]
    pub folder_id: Option<u32>,
}
