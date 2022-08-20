use rocket::fs::TempFile;
use rocket::serde::Deserialize;
use rocket::FromForm;

#[derive(FromForm)]
pub struct FileUpload<'a> {
    pub file: TempFile<'a>,
    pub extension: &'a str,
}

/// Because `Auth` is used as a request guard, we can't use it for creating login credentials.
/// This allows us to accept one in a post body.
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewAuth {
    pub username: String,
    pub password: String,
}
