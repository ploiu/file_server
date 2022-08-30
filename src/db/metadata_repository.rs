use rusqlite::Connection;

use crate::guard::Auth;

/// returns the current version of the database as a String
pub fn get_version(con: &mut Connection) -> rusqlite::Result<String> {
    let result = con.query_row(
        include_str!("../assets/queries/metadata/get_api_version.sql"),
        [],
        |row| row.get(0),
    );
    return result;
}

/// represents the result of comparing a password to the database value
pub enum CheckAuthResult {
    /// The passed authorization matches what's in the database
    Valid,
    /// The passed authorization does not match what's in the database
    Invalid,
    /// there is no auth field in the database, and one needs to be set
    Missing,
}

/// retrieves the encrypted authentication string for requests in the database
pub fn get_auth(con: &mut Connection) -> rusqlite::Result<String> {
    con.query_row(
        include_str!("../assets/queries/metadata/get_auth_hash.sql"),
        [],
        |row| row.get(0),
    )
}

/// checks if the passed `auth` matches the encrypted auth string in the database
pub fn check_auth(auth: Auth, con: &mut Connection) -> CheckAuthResult {
    let hash = auth.to_string();
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

pub fn set_auth(auth: Auth, con: &mut Connection) -> bool {
    let mut statement = con
        .prepare(include_str!("../assets/queries/metadata/set_auth_hash.sql"))
        .unwrap();
    return match statement.execute([auth.to_string()]) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Failed to set password! Error is: \n{:?}", e);
            false
        }
    };
}
