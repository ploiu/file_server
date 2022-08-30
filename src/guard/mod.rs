use std::io::Write;

use base64::decode;
use rocket::async_trait;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use sha2::{Digest, Sha256};

use crate::db::metadata_repository::CheckAuthResult;
use crate::facade::db::check_auth;

/// used to represent the result of calling `Auth::validate`
pub enum ValidateResult {
    Ok,
    NoPasswordSet,
    Invalid,
}

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
                if split.len() != 2 || split.contains(&"") {
                    return Err("Invalid basic auth format: missing username or password");
                }
                Ok(Auth {
                    username: String::from(split[0].trim()),
                    password: String::from(split[1].trim()),
                })
            }
            Err(_) => Err("Invalid basic auth format: not base64"),
        }
    }

    /// compares our value with that in the database and returns a `Some` if the password doesn't match for any reason.
    ///
    /// _this is a convenience method to be used only in handlers_
    pub fn validate(self) -> ValidateResult {
        match check_auth(self) {
            CheckAuthResult::Valid => ValidateResult::Ok,
            CheckAuthResult::Missing => ValidateResult::NoPasswordSet,
            CheckAuthResult::Invalid => ValidateResult::Invalid,
        }
    }

    pub fn to_string(&self) -> String {
        let mut hasher = Sha256::new();
        // hash username and password combined
        let combined = format!("{}:{}", self.username.trim(), self.password.trim());
        hasher.write(combined.as_bytes()).unwrap();
        format!("{:x}", hasher.finalize())
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
            None => Outcome::Failure((Status::Unauthorized, AuthError::Missing)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_valid_input() {
        // test:test
        let input = "Basic dGVzdDp0ZXN0Cg==";
        let output = Auth::from(input).unwrap();
        assert_eq!("test", output.username);
        assert_eq!("test", output.password);
    }

    #[test]
    fn test_from_unencoded_input() {
        let input = "test:test";
        let output = Auth::from(input).unwrap_err();
        assert_eq!("Invalid basic auth format: not base64", output);
    }

    #[test]
    fn test_from_bad_input() {
        // :test
        assert_eq!(
            "Invalid basic auth format: missing username or password",
            Auth::from("OnRlc3Q=").unwrap_err()
        );
        // test:
        assert_eq!(
            "Invalid basic auth format: missing username or password",
            Auth::from("dGVzdDo=").unwrap_err()
        );
        // testtest
        assert_eq!(
            "Invalid basic auth format: missing username or password",
            Auth::from("dGVzdHRlc3Q=").unwrap_err()
        )
    }

    #[test]
    fn test_to_string() {
        let auth = Auth {
            username: "test".to_string(),
            password: "test".to_string(),
        };
        assert_eq!(
            "31f014b53e5861c8b28a8707a1d6a2a2737ce2c22fd671884173498510a063f0",
            auth.to_string()
        );
    }
}
