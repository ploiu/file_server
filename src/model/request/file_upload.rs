use rocket::FromForm;
use rocket::fs::TempFile;

#[derive(FromForm)]
pub struct FileUpload<'a> {
    pub file: TempFile<'a>,
    pub extension: &'a str
}