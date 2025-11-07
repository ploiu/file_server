use std::backtrace::Backtrace;
use std::collections::HashSet;

use crate::model::error::file_errors::GetFileError;
use crate::model::error::tag_errors::{
    CreateTagError, DeleteTagError, GetTagError, TagRelationError, UpdateTagError,
};
use crate::model::repository;
use crate::model::response::TagApi;
use crate::repository::{open_connection, tag_repository};
use crate::service::{file_service, folder_service};

/// will create a tag, or return the already-existing tag if one with the same name exists
/// returns the created/existing tag
pub fn create_tag(name: String) -> Result<TagApi, CreateTagError> {
    let con = open_connection();
    let existing_tag: Option<repository::Tag> = match tag_repository::get_tag_by_title(&name, &con)
    {
        Ok(tags) => tags,
        Err(e) => {
            log::error!(
                "Failed to check if any tags with the name {name} already exist! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(CreateTagError::DbError);
        }
    };
    let tag: repository::Tag = if let Some(t) = existing_tag {
        t
    } else {
        match tag_repository::create_tag(&name, &con) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to create a new tag with the name {name}! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(CreateTagError::DbError);
            }
        }
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
            log::error!(
                "No tag with id {id} exists!\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(GetTagError::TagNotFound);
        }
        Err(e) => {
            log::error!(
                "Could not retrieve tag with id {id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
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
    // make sure the tag exists first TODO cleanup - use if let Err pattern since Ok branch is empty
    match tag_repository::get_tag(request.id.unwrap(), &con) {
        Ok(_) => { /* no op */ }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            log::error!(
                "Could not update tag with id {:?}, because it does not exist!\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::TagNotFound);
        }
        Err(e) => {
            log::error!(
                "Could not update tag with id {:?}! Error is {e}\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    let new_title = request.title;
    // now make sure the database doesn't already have a tag with the new name TODO maybe see if can clean up, 2 empty branches is a smell
    match tag_repository::get_tag_by_title(&new_title, &con) {
        Ok(Some(_)) => {
            log::error!(
                "Could not update tag with id {:?} to name {new_title}, because a tag with that name already exists!\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::NewNameAlreadyExists);
        }
        Ok(None) => {}
        Err(rusqlite::Error::QueryReturnedNoRows) => { /* this is the good route - no op */ }
        Err(e) => {
            log::error!(
                "Could not search tags by name with value {new_title}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    // no match, and tag already exists so we're good to go
    let db_tag = repository::Tag {
        id: request.id.unwrap(),
        title: new_title.clone(),
    };
    match tag_repository::update_tag(db_tag, &con) {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Could not update tag with id {:?}! Error is {e}\n{}",
                request.id,
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(UpdateTagError::DbError);
        }
    };
    con.close().unwrap();
    Ok(TagApi {
        id: request.id,
        title: new_title,
    })
}

/// deletes the tag with the passed id. Does nothing if that tag doesn't exist
pub fn delete_tag(id: u32) -> Result<(), DeleteTagError> {
    let con: rusqlite::Connection = open_connection();
    // TODO change to if let Err pattern, Ok branch is empty
    match tag_repository::delete_tag(id, &con) {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Could not delete tag with id {id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(DeleteTagError::DbError);
        }
    };
    con.close().unwrap();
    Ok(())
}

/// Updates the tags on a file by replacing all existing tags with the provided list.
///
/// This function will:
/// 1. Remove all existing tags from the file
/// 2. Add tags that already exist in the database (those with an `id`)
/// 3. Create and add new tags (those without an `id`)
///
/// Duplicate tags in the input list will be automatically deduplicated to prevent
/// database constraint violations.
///
/// # Parameters
/// - `file_id`: The ID of the file to update tags for
/// - `tags`: A vector of tags to set on the file. Tags with an `id` will be linked directly,
///   tags without an `id` will be created first (or retrieved if they already exist by name)
///
/// # Returns
/// - `Ok(())` if the tags were successfully updated
/// - `Err(TagRelationError::FileNotFound)` if the file does not exist
/// - `Err(TagRelationError::DbError)` if there was a database error
pub fn update_file_tags(file_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    // make sure the file exists
    if Err(GetFileError::NotFound) == file_service::get_file_metadata(file_id) {
        log::error!(
            "Cannot update tag for file {file_id}, because that file does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let existing_tags = get_tags_on_file(file_id)?;
    let con: rusqlite::Connection = open_connection();
    // Remove all existing tags from the file
    for tag in existing_tags.iter() {
        // tags from the db will always have a non-None tag id
        if let Err(e) = tag_repository::remove_tag_from_file(file_id, tag.id.unwrap(), &con) {
            log::error!(
                "Failed to remove tag from file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    }

    // Track which tag IDs have been added to avoid duplicates
    let mut added_tag_ids: HashSet<u32> = HashSet::new();

    // First, add all existing tags (those with an id)
    let existing_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_some()).collect();
    for tag in existing_tags {
        let tag_id = tag.id.unwrap();
        // Skip if we've already added this tag
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_file(file_id, tag_id, &con) {
            log::error!(
                "Failed to add tag to file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    // Then, create and add new tags (those without an id)
    let new_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_none()).collect();
    for tag in new_tags {
        let created_tag = match create_tag(tag.title.clone()) {
            Ok(t) => t,
            Err(_) => {
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        };
        let tag_id = created_tag.id.unwrap();
        // Skip if we've already added this tag (prevents duplicates)
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_file(file_id, tag_id, &con) {
            log::error!(
                "Failed to add tag to file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    con.close().unwrap();
    Ok(())
}

/// Updates the tags on a folder by replacing all existing tags with the provided list.
///
/// This function will:
/// 1. Remove all existing tags from the folder
/// 2. Add tags that already exist in the database (those with an `id`)
/// 3. Create and add new tags (those without an `id`)
///
/// Duplicate tags in the input list will be automatically deduplicated to prevent
/// database constraint violations.
///
/// # Parameters
/// - `folder_id`: The ID of the folder to update tags for
/// - `tags`: A vector of tags to set on the folder. Tags with an `id` will be linked directly,
///   tags without an `id` will be created first (or retrieved if they already exist by name)
///
/// # Returns
/// - `Ok(())` if the tags were successfully updated
/// - `Err(TagRelationError::FolderNotFound)` if the folder does not exist
/// - `Err(TagRelationError::DbError)` if there was a database error
pub fn update_folder_tags(folder_id: u32, tags: Vec<TagApi>) -> Result<(), TagRelationError> {
    // make sure the file exists
    if !folder_service::folder_exists(Some(folder_id)) {
        log::error!("Cannot update tags for a folder that does not exist (id {folder_id}!");
        return Err(TagRelationError::FolderNotFound);
    }
    let existing_tags = get_tags_on_folder(folder_id)?;
    let con: rusqlite::Connection = open_connection();
    // Remove all existing tags from the folder
    for tag in existing_tags.iter() {
        // tags from the db will always have a non-None tag id
        if let Err(e) = tag_repository::remove_tag_from_folder(folder_id, tag.id.unwrap(), &con) {
            log::error!(
                "Failed to remove tags from folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    }

    // Track which tag IDs have been added to avoid duplicates
    let mut added_tag_ids: HashSet<u32> = HashSet::new();

    // First, add all existing tags (those with an id)
    let existing_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_some()).collect();
    for tag in existing_tags {
        let tag_id = tag.id.unwrap();
        // Skip if we've already added this tag
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_folder(folder_id, tag_id, &con) {
            log::error!(
                "Failed to add tags to folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    // Then, create and add new tags (those without an id)
    let new_tags: Vec<&TagApi> = tags.iter().filter(|t| t.id.is_none()).collect();
    for tag in new_tags {
        let created_tag = match create_tag(tag.title.clone()) {
            Ok(t) => t,
            Err(e) => {
                log::error!(
                    "Failed to create tag! Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return Err(TagRelationError::DbError);
            }
        };
        let tag_id = created_tag.id.unwrap();
        // Skip if we've already added this tag (prevents duplicates)
        if added_tag_ids.contains(&tag_id) {
            continue;
        }
        if let Err(e) = tag_repository::add_tag_to_folder(folder_id, tag_id, &con) {
            log::error!(
                "Failed to add tags to folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
        added_tag_ids.insert(tag_id);
    }

    con.close().unwrap();
    Ok(())
}

/// retrieves all the tags on the file with the passed id
pub fn get_tags_on_file(file_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    // make sure the file exists
    if !file_service::check_file_exists(file_id) {
        log::error!(
            "Cannot get tags on file with id {file_id}, because that file does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let con: rusqlite::Connection = open_connection();
    let file_tags = match tag_repository::get_tags_on_file(file_id, &con) {
        Ok(tags) => tags,
        Err(e) => {
            log::error!(
                "Failed to retrieve tags on file with id {file_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };
    con.close().unwrap();
    let api_tags: Vec<TagApi> = file_tags.into_iter().map(TagApi::from).collect();
    Ok(api_tags)
}

/// retrieves all the tags on the folder with the passed id.
/// This will always be empty if requesting with the root folder id (0 or None)
pub fn get_tags_on_folder(folder_id: u32) -> Result<Vec<TagApi>, TagRelationError> {
    // make sure the folder exists
    if !folder_service::folder_exists(Some(folder_id)) {
        log::error!(
            "Cannot get tags on folder with id {folder_id}, because that folder does not exist!\n{}",
            Backtrace::force_capture()
        );
        return Err(TagRelationError::FileNotFound);
    }
    let con: rusqlite::Connection = open_connection();
    let db_tags = match tag_repository::get_tags_on_folder(folder_id, &con) {
        Ok(tags) => tags,
        Err(e) => {
            log::error!(
                "Failed to retrieve tags on folder with id {folder_id}! Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return Err(TagRelationError::DbError);
        }
    };
    con.close().unwrap();
    let api_tags: Vec<TagApi> = db_tags.into_iter().map(TagApi::from).collect();
    Ok(api_tags)
}

#[cfg(test)]
mod get_tag_tests {
    use crate::model::error::tag_errors::GetTagError;
    use crate::service::tag_service::{create_tag, get_tag};
    use crate::test::*;

    #[test]
    fn test_get_tag() {
        init_db_folder();
        let expected = create_tag("test".to_string()).unwrap();
        let actual = get_tag(1).unwrap();
        assert_eq!(actual, expected);
        cleanup();
    }

    #[test]
    fn test_get_tag_non_existent() {
        init_db_folder();
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
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn update_tag_works() {
        init_db_folder();
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
        init_db_folder();
        let res = update_tag(TagApi {
            id: Some(1),
            title: "what".to_string(),
        });
        assert_eq!(UpdateTagError::TagNotFound, res.unwrap_err());
        cleanup();
    }

    #[test]
    fn update_tag_already_exists() {
        init_db_folder();
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
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn delete_tag_works() {
        init_db_folder();
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
    use crate::model::file_types::FileTypes;
    use crate::model::repository::FileRecord;
    use crate::model::response::TagApi;
    use crate::repository::{file_repository, open_connection};
    use crate::service::tag_service::{create_tag, get_tags_on_file, update_file_tags};
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn update_file_tags_works() {
        init_db_folder();
        create_tag("test".to_string()).unwrap();
        FileRecord {
            id: None,
            name: "test_file".to_string(),
            parent_id: None,
            size: 0,
            create_date: chrono::offset::Local::now().naive_local(),
            file_type: FileTypes::Unknown,
        }
        .save_to_db();
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
        init_db_folder();
        FileRecord {
            id: None,
            name: "test".to_string(),
            parent_id: None,
            size: 0,
            create_date: chrono::offset::Local::now().naive_local(),
            file_type: FileTypes::Unknown,
        }
        .save_to_db();
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
        init_db_folder();
        let res = update_file_tags(1, vec![]).unwrap_err();
        assert_eq!(TagRelationError::FileNotFound, res);
        cleanup();
    }

    #[test]
    fn update_file_tags_deduplicates_existing_tags() {
        init_db_folder();
        create_tag("test".to_string()).unwrap();
        FileRecord {
            id: None,
            name: "test_file".to_string(),
            parent_id: None,
            size: 0,
            create_date: chrono::offset::Local::now().naive_local(),
            file_type: FileTypes::Unknown,
        }
        .save_to_db();

        // Try to add the same tag twice - should not fail and should only add it once
        update_file_tags(
            1,
            vec![
                TagApi {
                    id: Some(1),
                    title: "test".to_string(),
                },
                TagApi {
                    id: Some(1),
                    title: "test".to_string(),
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_file(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].id, Some(1));
        assert_eq!(actual[0].title, "test");
        cleanup();
    }

    #[test]
    fn update_file_tags_deduplicates_new_tags_with_same_name() {
        init_db_folder();
        FileRecord {
            id: None,
            name: "test_file".to_string(),
            parent_id: None,
            size: 0,
            create_date: chrono::offset::Local::now().naive_local(),
            file_type: FileTypes::Unknown,
        }
        .save_to_db();

        // Create tag implicitly by name twice - should only create once
        update_file_tags(
            1,
            vec![
                TagApi {
                    id: None,
                    title: "test".to_string(),
                },
                TagApi {
                    id: None,
                    title: "test".to_string(),
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_file(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].id, Some(1));
        assert_eq!(actual[0].title, "test");
        cleanup();
    }

    #[test]
    fn update_file_tags_skips_duplicate_after_creating() {
        init_db_folder();
        FileRecord {
            id: None,
            name: "test_file".to_string(),
            parent_id: None,
            size: 0,
            create_date: chrono::offset::Local::now().naive_local(),
            file_type: FileTypes::Unknown,
        }
        .save_to_db();

        // Mix of new tag by name and existing tag by id (same tag)
        update_file_tags(
            1,
            vec![TagApi {
                id: None,
                title: "test".to_string(),
            }],
        )
        .unwrap();

        // Now update with both the id and a new tag with same name
        update_file_tags(
            1,
            vec![
                TagApi {
                    id: Some(1),
                    title: "test".to_string(),
                },
                TagApi {
                    id: None,
                    title: "test".to_string(),
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_file(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].id, Some(1));
        assert_eq!(actual[0].title, "test");
        cleanup();
    }
}

#[cfg(test)]
mod update_folder_tag_test {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::model::repository::Folder;
    use crate::model::response::TagApi;
    use crate::repository::{folder_repository, open_connection};
    use crate::service::tag_service::{create_tag, get_tags_on_folder, update_folder_tags};
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn update_folder_tags_works() {
        init_db_folder();
        let con = open_connection();
        create_tag("test".to_string()).unwrap();
        folder_repository::create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_file".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();
        update_folder_tags(
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
        let actual = get_tags_on_folder(1).unwrap();
        assert_eq!(actual, expected);
        cleanup();
    }

    #[test]
    fn update_folder_tags_removes_tags() {
        init_db_folder();
        let con = open_connection();
        folder_repository::create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();
        update_folder_tags(
            1,
            vec![TagApi {
                id: None,
                title: "test".to_string(),
            }],
        )
        .unwrap();
        update_folder_tags(1, vec![]).unwrap();
        assert_eq!(get_tags_on_folder(1).unwrap(), vec![]);
        cleanup();
    }

    #[test]
    fn update_folder_tags_throws_error_if_folder_not_found() {
        init_db_folder();
        let res = update_folder_tags(1, vec![]).unwrap_err();
        assert_eq!(TagRelationError::FolderNotFound, res);
        cleanup();
    }

    #[test]
    fn update_folder_tags_deduplicates_existing_tags() {
        init_db_folder();
        let con = open_connection();
        create_tag("test".to_string()).unwrap();
        folder_repository::create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();

        // Try to add the same tag twice - should not fail and should only add it once
        update_folder_tags(
            1,
            vec![
                TagApi {
                    id: Some(1),
                    title: "test".to_string(),
                },
                TagApi {
                    id: Some(1),
                    title: "test".to_string(),
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_folder(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].id, Some(1));
        assert_eq!(actual[0].title, "test");
        cleanup();
    }

    #[test]
    fn update_folder_tags_deduplicates_new_tags_with_same_name() {
        init_db_folder();
        let con = open_connection();
        folder_repository::create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();

        // Create tag implicitly by name twice - should only create once
        update_folder_tags(
            1,
            vec![
                TagApi {
                    id: None,
                    title: "test".to_string(),
                },
                TagApi {
                    id: None,
                    title: "test".to_string(),
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_folder(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].id, Some(1));
        assert_eq!(actual[0].title, "test");
        cleanup();
    }

    #[test]
    fn update_folder_tags_skips_duplicate_after_creating() {
        init_db_folder();
        let con = open_connection();
        folder_repository::create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();

        // Mix of new tag by name and existing tag by id (same tag)
        update_folder_tags(
            1,
            vec![TagApi {
                id: None,
                title: "test".to_string(),
            }],
        )
        .unwrap();

        // Now update with both the id and a new tag with same name
        update_folder_tags(
            1,
            vec![
                TagApi {
                    id: Some(1),
                    title: "test".to_string(),
                },
                TagApi {
                    id: None,
                    title: "test".to_string(),
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_folder(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].id, Some(1));
        assert_eq!(actual[0].title, "test");
        cleanup();
    }
}

#[cfg(test)]
mod get_tags_on_file_tests {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::service::tag_service::get_tags_on_file;
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn throws_error_if_file_not_found() {
        init_db_folder();
        let err = get_tags_on_file(1).unwrap_err();
        assert_eq!(TagRelationError::FileNotFound, err);
        cleanup();
    }
}

#[cfg(test)]
mod get_tags_on_folder_tests {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::service::tag_service::get_tags_on_folder;
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn throws_error_if_file_not_found() {
        init_db_folder();
        let err = get_tags_on_folder(1).unwrap_err();
        assert_eq!(TagRelationError::FileNotFound, err);
        cleanup();
    }
}
