use crate::facade::db::{is_password_set, set_password};
use crate::guard::Auth;
use crate::model::request::NewAuth;

pub fn create_password<'a>(auth: NewAuth) -> Result<(), &'a str> {
    if is_password_set() {
        return Err("password cannot be set, as it already has been set");
    }
    let auth = Auth {
        username: auth.username,
        password: auth.password,
    };
    return if set_password(auth) {
        Ok(())
    } else {
        Err("failed to set password")
    };
}
