use crate::model::error::tag_errors::{
    CreateTagError, DeleteTagError, GetTagError, TagRelationError, UpdateTagError,
};
use crate::model::repository;
use crate::model::response::TagApi;
use crate::repository::{open_connection, tag_repository};

/// will create a tag, or return the already-existing tag if one with the same name exists
/// returns the created/existing tag
// TODO test
pub fn create_tag(name: String) -> Result<TagApi, CreateTagError> {
    let con = open_connection();
    let copied_name = name.clone();
    let existing_tag: Option<repository::Tag> = match tag_repository::get_tag_by_title(name, &con) {
        Ok(tags) => tags,
        Err(_) => {
            con.close().unwrap();
            return Err(CreateTagError::DbError);
        }
    };
    let tag: repository::Tag = if None == existing_tag {
        match tag_repository::create_tag(copied_name, &con) {
            Ok(t) => t,
            Err(_) => {
                con.close().unwrap();
                return Err(CreateTagError::DbError);
            }
        }
    } else {
        existing_tag.unwrap()
    };
    con.close().unwrap();
    Ok(TagApi::from(tag))
}

/// will return the tag with the passed id
// TODO test
pub fn get_tag(id: u32) -> Result<TagApi, GetTagError> {
    let con = open_connection();
    let tag: repository::Tag = match tag_repository::get_tag(id, &con) {
        Ok(t) => t,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            con.close().unwrap();
            return Err(GetTagError::TagNotFound);
        }
        Err(_) => {
            con.close().unwrap();
            return Err(GetTagError::DbError);
        }
    };
    con.close().unwrap();
    Ok(TagApi::from(tag))
}

/// updates the tag with the passed id to the passed name.
/// Will fail if a tag already exists with that name
pub fn update_tag(request: TagApi) -> Result<TagApi, UpdateTagError> {
    panic!("unimplemented");
}

/// deletes the tag with the passed id. Does nothing if that tag doesn't exist
pub fn delete_tag(id: u32) -> Result<(), DeleteTagError> {
    panic!("unimplemented");
}

/// removes all the tags on the file with the passed id and sets them all to be the passed list
pub fn update_file_tags(file_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    panic!("unimplemented");
}

/// removes all the tags on the folder with the passed id and sets them all to be the passed list
pub fn update_folder_tags(folder_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    panic!("unimplemented");
}

/// retrieves all the tags on the file with the passed id
pub fn get_tags_on_file(file_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    panic!("unimplemented");
}

/// retrieves all the tags on the folder with the passed id.
/// This will always be empty if requesting with the root folder id (0 or None)
pub fn get_tags_on_folder(folder_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    panic!("unimplemented");
}
