use crate::db::metadata::CheckAuthResult;
use crate::facade::db::check_auth;
use crate::model::response::BasicResponse;
use base64::decode;
use rocket::async_trait;
use rocket::http::{ContentType, MediaType, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

#[derive(Debug)]
pub struct Auth {
    pub username: String,
    pub password: String,
}

impl Auth {
    /// creates an `Auth` object from the passed header value.
    /// The value of header must be base64-encoded basic auth.
    pub fn from(header: &str) -> Result<Auth, &str> {
        // remove the "Basic " from the header, leaving only the base64 part
        let stripped_header = header.to_string().replace("Basic ", "");
        match decode(stripped_header.as_str()) {
            Ok(value) => {
                let combined = String::from_utf8(value).unwrap();
                let split = combined.split(":").collect::<Vec<&str>>();
                // if there aren't exactly 2 parts, then something is wrong here
                if split.len() != 2 {
                    return Err("Invalid basic auth format");
                }
                Ok(Auth {
                    username: String::from(split[0]),
                    password: String::from(split[1]),
                })
            }
            Err(_) => Err("Invalid basic auth format"),
        }
    }
    pub fn validate<'a>(self) -> Option<(Status, BasicResponse<'a>)> {
        match check_auth(self) {
            CheckAuthResult::Valid => None,
            CheckAuthResult::Invalid => {
                Some(BasicResponse::text(Status::Unauthorized, "Bad credentials"))
            }
            //language=json
            CheckAuthResult::Missing => Some(BasicResponse::json(
                Status::BadRequest,
                "{\"message\": \"No password set. Please set one via `/password`\"}",
            )),
        }
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for Auth {
    type Error = AuthError;

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        // just check if it's basic auth
        fn check_basic_auth(value: &str) -> bool {
            String::from(value).starts_with("Basic")
        }
        match request.headers().get_one("Authorization") {
            None => Outcome::Failure((Status::BadRequest, AuthError::Missing)),
            Some(value) if check_basic_auth(value) => match Auth::from(value) {
                // TODO check against db
                Ok(auth) => Outcome::Success(auth),
                Err(_) => Outcome::Failure((Status::Unauthorized, AuthError::Invalid)),
            },
            Some(_) => Outcome::Failure((Status::BadRequest, AuthError::Invalid)),
        }
    }
}

#[derive(Debug)]
pub enum AuthError {
    Missing,
    Invalid,
}
