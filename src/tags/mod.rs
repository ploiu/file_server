pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

/// lists the different types of tags that can exist on a file or folder
#[derive(Eq, PartialEq)]
enum TagTypes {
    /// The tag was individually set on the file or folder
    Explicit,
    /// the tag was individually set on an ancestor folder
    Implicit,
}
