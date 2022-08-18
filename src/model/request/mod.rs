use rocket::fs::TempFile;
use rocket::FromForm;

#[derive(FromForm)]
pub struct FileUpload<'a> {
    pub file: TempFile<'a>,
    pub extension: &'a str,
}
