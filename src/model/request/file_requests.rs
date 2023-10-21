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
    ///
    /// so it _appears_ that Rocket has trouble parsing form data fields if they're a number,
    /// and turning it into a String fixes it.
    /// Weird thing is, it works from postman and curl, but not from javascript form body,
    /// intellij http scratch pad (even directly imported from curl), or java.
    /// I don't want to pursue this anymore, and this works
    folder_id: Option<String>,
}

impl CreateFileRequest<'_> {
    pub fn folder_id(&self) -> u32 {
        match &self.folder_id {
            Some(id) => id.to_string().parse::<u32>(),
            None => Ok(0),
        }
        .unwrap()
    }
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateFileRequest {
    pub id: u32,
    pub name: String,
    #[serde(rename = "folderId")]
    pub folder_id: Option<u32>,
}
