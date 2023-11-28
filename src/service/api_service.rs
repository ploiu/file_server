use crate::guard::Auth;
use crate::model::error::metadata_errors::CreatePasswordError;
use crate::model::request::NewAuth;
use crate::model::service::metadata::CheckAuthResult;
use crate::repository;
use crate::repository::metadata_repository;

pub fn create_password(auth: NewAuth) -> Result<(), CreatePasswordError> {
    if is_password_set() {
        return Err(CreatePasswordError::AlreadyExists);
    }
    let auth = Auth {
        username: auth.username,
        password: auth.password,
    };
    if set_password(auth) {
        Ok(())
    } else {
        Err(CreatePasswordError::Failure)
    }
}

/// Checks if the passed `auth` object matches the password in the database
pub fn check_auth(auth: Auth) -> CheckAuthResult {
    let mut con = repository::open_connection();
    let result = metadata_repository::check_auth(auth, &mut con);
    con.close().unwrap();
    if let Ok(r) = result {
        r
    } else {
        CheckAuthResult::DbError
    }
}

// private functions

/// checks if a password was set in the database
fn is_password_set() -> bool {
    let mut con = repository::open_connection();
    let auth_result = metadata_repository::get_auth(&mut con);
    con.close().unwrap();

    match auth_result {
        Ok(_) => true,
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => false,
        Err(e) => {
            panic!("Failed to check auth in database: {:?}", e);
        }
    }
}

/// saves the passed auth to the database.
///
/// This should never be called if there is a password already set (see `is_password_set`), because
/// it will override whatever is already in the database.
fn set_password(auth: Auth) -> bool {
    let mut con = repository::open_connection();
    let result = metadata_repository::set_auth(auth, &mut con);
    con.close().unwrap();
    result.is_ok()
}
