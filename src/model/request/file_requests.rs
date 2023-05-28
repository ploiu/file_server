use core::option::Option;

use rocket::fs::TempFile;
use rocket::serde::{Deserialize, Serialize};

#[derive(FromForm)]
pub struct CreateFileRequest<'a> {
    /// the file being uploaded
    pub file: TempFile<'a>,
    /// because I don't feel like mapping from content-type header
    /// TODO look at TempFile#raw_name, and when a TempFile is a Buffer vs a File (raw_name will be None if it's a Buffer)
    pub extension: Option<String>,
    /// leave blank for top level folder TODO rename to folderId in api
    pub folder_id: Option<u32>,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateFileRequest {
    pub id: u32,
    pub name: String,
    #[serde(rename = "folderId")]
    pub folder_id: Option<u32>,
}
