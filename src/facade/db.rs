use crate::db::metadata_repository;
use crate::db::metadata_repository::{get_auth, set_auth, CheckAuthResult};
use crate::db::open_connection;
use crate::guard::Auth;

/// Checks if the passed `auth` object matches the password in the database
pub fn check_auth(auth: Auth) -> CheckAuthResult {
    let mut con = open_connection();
    let result = metadata_repository::check_auth(auth, &mut con);
    con.close().unwrap();
    result
}

/// checks if a password was set in the database
pub fn is_password_set() -> bool {
    let mut con = open_connection();
    let has_password = match get_auth(&mut con) {
        Ok(_) => true,
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => false,
        Err(e) => {
            con.close().unwrap();
            panic!("Failed to check auth in database: {:?}", e);
        }
    };
    con.close().unwrap();
    has_password
}

/// saves the passed auth to the database.
///
/// This should never be called if there is a password already set (see `is_password_set`), because
/// it will override whatever is already in the database.
pub fn set_password(auth: Auth) -> bool {
    let mut con = open_connection();
    let result = set_auth(auth, &mut con);
    con.close().unwrap();
    result
}
