use crate::facade::metadata_facade::{is_password_set, set_password};
use crate::guard::Auth;
use crate::model::request::NewAuth;

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
