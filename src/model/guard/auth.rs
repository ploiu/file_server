/// used to represent the result of calling `Auth::validate`
pub enum ValidateResult {
    Ok,
    NoPasswordSet,
    Invalid,
}
