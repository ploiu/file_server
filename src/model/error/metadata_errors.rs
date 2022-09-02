#[derive(PartialEq)]
pub enum CreatePasswordError {
    AlreadyExists,
    Failure,
}
