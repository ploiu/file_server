use std::fmt;
use std::io::Write;

use rocket::serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::guard::HeaderAuth;

pub mod file_requests;
pub mod folder_requests;

/// Because `Auth` is used as a request guard, we can't use it for creating login credentials.
/// This allows us to accept one in a post body.
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct BodyAuth {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateAuth {
    /// the old username + password pair
    #[serde(rename = "oldAuth")]
    pub old_auth: BodyAuth,
    /// the new username + password pair
    #[serde(rename = "newAuth")]
    pub new_auth: BodyAuth,
}

impl fmt::Display for BodyAuth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hasher = Sha256::new();
        // hash username and password combined
        let combined = format!("{}:{}", self.username.trim(), self.password.trim());
        hasher.write_all(combined.as_bytes()).unwrap();
        write!(f, "{:x}", hasher.finalize())
    }
}

impl BodyAuth {
    pub fn into_auth(self) -> HeaderAuth {
        HeaderAuth {
            username: self.username,
            password: self.password,
        }
    }
}
