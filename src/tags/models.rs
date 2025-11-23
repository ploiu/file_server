/// lists the different types of tags that can exist on a file or folder
#[derive(Eq, PartialEq)]
pub enum TagTypes {
    /// The tag was individually set on the file or folder
    Explicit,
    /// the tag was individually set on an ancestor folder
    Implicit,
}

/// represents a tag in the Tags table of the database. When referencing a tag _on_ a file / folder, use [`TaggedItem`] instead
#[derive(Debug, PartialEq, Clone)]
pub struct Tag {
    /// the id of the tag
    pub id: u32,
    /// the display name of the tag
    pub title: String,
}

/// represents a tag on a file or a folder, with optional implication.
/// These are not meant to ever be created outside of a database query retrieving it from the database
///
/// [`file_id`] _or_ [`folder_id`] will be [`None`], but never both. [`implicit_from_id`] will be None if the tag is explicitly on the item
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TaggedItem {
    /// the database id of this specific entry
    pub id: u32,
    /// if present, the id of the file this tag exists on. mutually exclusive with folder_id
    pub file_id: Option<u32>,
    /// if present, the id of the folder this tag exists on. mutually exclusive with file_id
    pub folder_id: Option<u32>,
    /// if present, the folder that implicates this tag on the file/folder this tag applies to
    pub implicit_from_id: Option<u32>,
    /// the tag's title
    pub title: String,
    /// the id of the actual tag
    pub tag_id: u32,
}
