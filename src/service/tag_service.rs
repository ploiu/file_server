use crate::model::error::file_errors::GetFileError;
use crate::model::error::tag_errors::{
    CreateTagError, DeleteTagError, GetTagError, TagRelationError, UpdateTagError,
};
use crate::model::repository;
use crate::model::repository::Tag;
use crate::model::response::TagApi;
use crate::repository::{open_connection, tag_repository};
use crate::service::file_service;

/// will create a tag, or return the already-existing tag if one with the same name exists
/// returns the created/existing tag
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
    let con: rusqlite::Connection = open_connection();
    // make sure the tag exists first
    match tag_repository::get_tag(request.id.unwrap(), &con) {
        Ok(_) => { /* no op */ }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            con.close().unwrap();
            return Err(UpdateTagError::TagNotFound);
        }
        Err(_) => {
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    let new_title = request.title;
    // now make sure the database doesn't already have a tag with the new name
    match tag_repository::get_tag_by_title(new_title.clone(), &con) {
        Ok(Some(_)) => {
            con.close().unwrap();
            return Err(UpdateTagError::NewNameAlreadyExists);
        }
        Ok(None) => {}
        Err(rusqlite::Error::QueryReturnedNoRows) => { /* this is the good route - no op */ }
        Err(_) => {
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    // no match, and tag already exists so we're good to go
    let db_tag = Tag {
        id: request.id.unwrap(),
        title: new_title.clone(),
    };
    match tag_repository::update_tag(db_tag, &con) {
        Ok(()) => {}
        Err(_) => {
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    con.close().unwrap();
    return Ok(TagApi {
        id: request.id,
        title: new_title.clone(),
    });
}

/// deletes the tag with the passed id. Does nothing if that tag doesn't exist
pub fn delete_tag(id: u32) -> Result<(), DeleteTagError> {
    let con: rusqlite::Connection = open_connection();
    match tag_repository::delete_tag(id, &con) {
        Ok(()) => {}
        Err(_) => {
            con.close().unwrap();
            return Err(DeleteTagError::DbError);
        }
    };
    con.close().unwrap();
    Ok(())
}

/// removes all the tags on the file with the passed id and sets them all to be the passed list
pub fn update_file_tags(file_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    // make sure the file exists
    if Err(GetFileError::NotFound) == file_service::get_file_metadata(file_id) {
        return Err(TagRelationError::FileNotFound);
    }
    let existing_tags = get_tags_on_file(file_id)?;
    let con: rusqlite::Connection = open_connection();
    for tag in existing_tags.iter() {
        // tags from the db will always have a non-None tag id
        match tag_repository::remove_tag_from_file(file_id, tag.id.unwrap(), &con) {
            Ok(()) => {}
            Err(_) => {
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        }
    }
    // for all the new tags, create them first
    let new_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_none()).collect();
    for tag in new_tags {
        let created_tag = match tag_repository::create_tag(tag.title.clone(), &con) {
            Ok(t) => t,
            Err(_) => {
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        };
        match tag_repository::add_tag_to_file(file_id, created_tag.id, &con) {
            Ok(()) => {}
            Err(_) => {
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        }
    }
    let existing_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_some()).collect();
    for tag in existing_tags {
        match tag_repository::add_tag_to_file(file_id, tag.id.unwrap(), &con) {
            Ok(()) => {}
            Err(_) => {
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        }
    }
    con.close().unwrap();
    Ok(())
}

/// removes all the tags on the folder with the passed id and sets them all to be the passed list
pub fn update_folder_tags(folder_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    panic!("unimplemented");
}

/// retrieves all the tags on the file with the passed id
pub fn get_tags_on_file(file_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    // make sure the file exists
    if Err(GetFileError::NotFound) == file_service::get_file_metadata(file_id) {
        return Err(TagRelationError::FileNotFound);
    }
    let con: rusqlite::Connection = open_connection();
    let db_tags = match tag_repository::get_tags_on_file(file_id, &con) {
        Ok(tags) => tags,
        Err(_) => {
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };
    con.close().unwrap();
    let api_tags: Vec<TagApi> = db_tags.into_iter().map(TagApi::from).collect();
    Ok(api_tags)
}

/// retrieves all the tags on the folder with the passed id.
/// This will always be empty if requesting with the root folder id (0 or None)
pub fn get_tags_on_folder(folder_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    panic!("unimplemented");
}

#[cfg(test)]
mod get_tag_tests {
    use crate::model::error::tag_errors::GetTagError;
    use crate::service::tag_service::{create_tag, get_tag};
    use crate::test::*;

    #[test]
    fn test_get_tag() {
        refresh_db();
        let expected = create_tag("test".to_string()).unwrap();
        let actual = get_tag(1).unwrap();
        assert_eq!(actual, expected);
        cleanup();
    }

    #[test]
    fn test_get_tag_non_existent() {
        refresh_db();
        let res = get_tag(1).expect_err("Retrieving a nonexistent tag should return an error");
        assert_eq!(GetTagError::TagNotFound, res);
        cleanup();
    }
}

#[cfg(test)]
mod update_tag_tests {
    use crate::model::error::tag_errors::UpdateTagError;
    use crate::model::response::TagApi;
    use crate::service::tag_service::{create_tag, get_tag, update_tag};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn update_tag_works() {
        refresh_db();
        let tag = create_tag("test_tag".to_string()).unwrap();
        let updated_tag = update_tag(TagApi {
            id: tag.id,
            title: "new_name".to_string(),
        })
        .unwrap();
        assert_eq!(String::from("new_name"), updated_tag.title);
        assert_eq!(Some(1), updated_tag.id);
        // test that it's in the database
        let updated_tag = get_tag(1).unwrap();
        assert_eq!(String::from("new_name"), updated_tag.title);
        cleanup();
    }

    #[test]
    fn update_tag_not_found() {
        refresh_db();
        let res = update_tag(TagApi {
            id: Some(1),
            title: "what".to_string(),
        });
        assert_eq!(UpdateTagError::TagNotFound, res.unwrap_err());
        cleanup();
    }

    #[test]
    fn update_tag_already_exists() {
        refresh_db();
        create_tag("first".to_string()).unwrap();
        create_tag("second".to_string()).unwrap();
        let res = update_tag(TagApi {
            id: Some(2),
            title: "FiRsT".to_string(),
        });
        assert_eq!(UpdateTagError::NewNameAlreadyExists, res.unwrap_err());
        cleanup();
    }
}

#[cfg(test)]
mod delete_tag_tests {
    use crate::model::error::tag_errors::GetTagError;
    use crate::service::tag_service::{create_tag, delete_tag, get_tag};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn delete_tag_works() {
        refresh_db();
        create_tag("test".to_string()).unwrap();
        delete_tag(1).unwrap();
        let res = get_tag(1).unwrap_err();
        assert_eq!(GetTagError::TagNotFound, res);
        cleanup();
    }
}

#[cfg(test)]
mod update_file_tag_test {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::model::repository::FileRecord;
    use crate::model::response::TagApi;
    use crate::repository::{file_repository, open_connection};
    use crate::service::tag_service::{create_tag, get_tags_on_file, update_file_tags};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn update_file_tags_works() {
        refresh_db();
        let con = open_connection();
        create_tag("test".to_string()).unwrap();
        file_repository::create_file(
            &FileRecord {
                id: None,
                name: "test_file".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();
        update_file_tags(
            1,
            vec![
                TagApi {
                    id: Some(1),
                    title: "test".to_string(),
                },
                TagApi {
                    id: None,
                    title: "new tag".to_string(),
                },
            ],
        )
        .unwrap();
        let expected = vec![
            TagApi {
                id: Some(1),
                title: "test".to_string(),
            },
            TagApi {
                id: Some(2),
                title: "new tag".to_string(),
            },
        ];
        let actual = get_tags_on_file(1).unwrap();
        assert_eq!(actual, expected);
        cleanup();
    }

    #[test]
    fn update_file_tags_removes_tags() {
        refresh_db();
        let con = open_connection();
        file_repository::create_file(
            &FileRecord {
                id: None,
                name: "test".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();
        update_file_tags(
            1,
            vec![TagApi {
                id: None,
                title: "test".to_string(),
            }],
        )
        .unwrap();
        update_file_tags(1, vec![]).unwrap();
        assert_eq!(get_tags_on_file(1).unwrap(), vec![]);
        cleanup();
    }

    #[test]
    fn update_file_tags_throws_error_if_file_not_found() {
        refresh_db();
        let res = update_file_tags(1, vec![]).unwrap_err();
        assert_eq!(TagRelationError::FileNotFound, res);
        cleanup();
    }
}

#[cfg(test)]
mod get_tags_on_file_tests {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::service::tag_service::get_tags_on_file;
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn throws_error_if_file_not_found() {
        refresh_db();
        let err = get_tags_on_file(1).unwrap_err();
        assert_eq!(TagRelationError::FileNotFound, err);
        cleanup();
    }
}
