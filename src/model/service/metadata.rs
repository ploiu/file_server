/// represents the result of comparing a password to the database value
#[derive(PartialEq, Debug)]
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
