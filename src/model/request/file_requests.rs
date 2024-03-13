use core::option::Option;

use rocket::fs::TempFile;

#[derive(FromForm)]
#[allow(non_snake_case)] // cannot serde rename the field, and it's better to have camel case for the api
pub struct CreateFileRequest<'a> {
    /// the file being uploaded
    pub file: TempFile<'a>,
    /// because I don't feel like mapping from content-type header
    pub extension: Option<String>,
    /// leave blank for top level folder
    ///
    /// so it _appears_ that Rocket has trouble parsing form data fields if they're a number,
    /// and turning it into a String fixes it.
    /// Weird thing is, it works from postman and curl, but not from javascript form body,
    /// intellij http scratch pad (even directly imported from curl), or java.
    /// I don't want to pursue this anymore, and this works
    folderId: Option<String>,
}

impl CreateFileRequest<'_> {
    pub fn folder_id(&self) -> u32 {
        match &self.folderId {
            Some(id) => id.to_string().parse::<u32>(),
            None => Ok(0),
        }
        .unwrap()
    }
}
