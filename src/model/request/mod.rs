pub mod file_requests;
pub mod folder_requests;

use rocket::serde::Deserialize;
/// Because `Auth` is used as a request guard, we can't use it for creating login credentials.
/// This allows us to accept one in a post body.
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewAuth {
    pub username: String,
    pub password: String,
}
