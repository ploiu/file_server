use crate::guard::Auth;
use rusqlite::Connection;
use sha2::digest::Mac;
use sha2::{Digest, Sha256};
use std::io::Write;

/// returns the current version of the database as a String
pub fn get_version(con: &mut Connection) -> rusqlite::Result<String> {
    let result = con.query_row(
        "select value from metadata where name = \"version\"",
        [],
        |row| row.get(0),
    );
    return result;
}

pub enum CheckAuthResult {
    /// The passed authorization matches what's in the database
    Valid,
    /// The passed authorization does not match what's in the database
    Invalid,
    /// there is no auth field in the database, and one needs to be set
    Missing,
}

fn get_auth(con: &mut Connection) -> rusqlite::Result<String> {
    con.query_row(
        "select value from Metadata where name = \"auth\"",
        [],
        |row| row.get(0),
    )
}

pub fn check_auth(auth: Auth, con: &mut Connection) -> CheckAuthResult {
    let mut hasher = Sha256::new();
    // hash username and password combined
    let combined = format!("{}:{}", auth.username, auth.password);
    hasher.write(combined.as_bytes()).unwrap();
    let hash = format!("{:x}", hasher.finalize());
    //language=sqlite
    let result = match get_auth(con) {
        Ok(db_hash) => {
            if db_hash.eq(&hash) {
                CheckAuthResult::Valid
            } else {
                CheckAuthResult::Invalid
            }
        }
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => CheckAuthResult::Missing,
        Err(e) => {
            panic!("Failed to check auth in database: {:?}", e);
        }
    };
    return result;
}
