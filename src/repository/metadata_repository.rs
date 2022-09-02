use rusqlite::Connection;

use crate::guard::Auth;

/// returns the current version of the database as a String
pub fn get_version(con: &mut Connection) -> Result<String, rusqlite::Error> {
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
    /// The database encountered an error trying to retrieve authorization
    DbError,
}

/// retrieves the encrypted authentication string for requests in the database
pub fn get_auth(con: &mut Connection) -> Result<String, rusqlite::Error> {
    con.query_row(
        include_str!("../assets/queries/metadata/get_auth_hash.sql"),
        [],
        |row| row.get(0),
    )
}

/// checks if the passed `auth` matches the encrypted auth string in the database
pub fn check_auth(auth: Auth, con: &mut Connection) -> Result<CheckAuthResult, rusqlite::Error> {
    let hash = auth.to_string();
    let result = match get_auth(con) {
        Ok(db_hash) => {
            if db_hash.eq(&hash) {
                Ok(CheckAuthResult::Valid)
            } else {
                Ok(CheckAuthResult::Invalid)
            }
        }
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Ok(CheckAuthResult::Missing),
        Err(e) => {
            eprintln!("Failed to check auth in database: {:?}", e);
            Err(e)
        }
    };
    return result;
}

pub fn set_auth(auth: Auth, con: &mut Connection) -> Result<(), rusqlite::Error> {
    let mut statement = con
        .prepare(include_str!("../assets/queries/metadata/set_auth_hash.sql"))
        .unwrap();
    return match statement.execute([auth.to_string()]) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to set password. Nested exception is {:?}", e);
            Err(e)
        }
    };
}
