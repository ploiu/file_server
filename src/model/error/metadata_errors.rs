#[derive(PartialEq, Debug)]
pub enum CreatePasswordError {
    AlreadyExists,
    Failure,
}

#[derive(PartialEq, Debug)]
pub enum UpdatePasswordError {
    Unauthorized,
}
