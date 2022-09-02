use crate::guard::Auth;
use crate::model::request::NewAuth;
use crate::repository;
use crate::repository::metadata_repository;
use crate::repository::metadata_repository::CheckAuthResult;

#[derive(PartialEq)]
pub enum CreatePasswordError {
    AlreadyExists,
    Failure,
}

pub fn create_password(auth: NewAuth) -> Result<(), CreatePasswordError> {
    if is_password_set() {
        return Err(CreatePasswordError::AlreadyExists);
    }
    let auth = Auth {
        username: auth.username,
        password: auth.password,
    };
    return if set_password(auth) {
        Ok(())
    } else {
        Err(CreatePasswordError::Failure)
    };
}

/// Checks if the passed `auth` object matches the password in the database
pub fn check_auth(auth: Auth) -> CheckAuthResult {
    let mut con = repository::open_connection();
    let result = metadata_repository::check_auth(auth, &mut con);
    con.close().unwrap();
    return if result.is_err() {
        CheckAuthResult::DbError
    } else {
        result.unwrap()
    };
}

// private functions

/// checks if a password was set in the database
fn is_password_set() -> bool {
    let mut con = repository::open_connection();
    let auth_result = metadata_repository::get_auth(&mut con);
    con.close().unwrap();
    let has_password = match auth_result {
        Ok(_) => true,
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => false,
        Err(e) => {
            panic!("Failed to check auth in database: {:?}", e);
        }
    };
    has_password
}

/// saves the passed auth to the database.
///
/// This should never be called if there is a password already set (see `is_password_set`), because
/// it will override whatever is already in the database.
fn set_password(auth: Auth) -> bool {
    let mut con = repository::open_connection();
    let result = metadata_repository::set_auth(auth, &mut con);
    con.close().unwrap();
    return !result.is_err();
}
