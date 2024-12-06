use std::backtrace::Backtrace;
use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use rusqlite::Connection;

use crate::model::api::FileApi;
use crate::model::error::file_errors::SearchFileError;
use crate::model::repository::FileRecord;
use crate::model::response::folder_responses::{FolderResponse};
use crate::model::response::TagApi;
use crate::repository::{file_repository, folder_repository, open_connection, tag_repository};
use crate::service::folder_service;

pub fn search_files(
    search_title: String,
    search_tags: Vec<String>,
) -> Result<HashSet<FileApi>, SearchFileError> {
    let search_tags: HashSet<String> = HashSet::from_iter(search_tags);
    let con: Connection = open_connection();
    let mut search_results: Vec<Result<HashSet<FileApi>, SearchFileError>> = vec![];
    if !search_tags.is_empty() {
        search_results.push(search_files_by_tags(&search_tags, &con).inspect_err(|e| {
            log::error!(
                "Failed to search files by tags. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            )
        }));
    }
    if !search_title.is_empty() {
        search_results.push(search_files_by_title(search_title, &con).inspect_err(|e| {
            log::error!(
                "Failed to search files by title. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            )
        }));
    }

    // TODO need to modularize this method because will be adding in searching on metadata. My idea is to perform each type of search
    // individually (tags, then metadata, then text), retain only the ones that intersect between all 3,
    // but pulling tags for search by tags is NECESSARY, so we can't pull tags at the end.
    //
    // I could make the retain function only care about looking at file IDs. idk your tests should cover any mistakes
    let res: Result<Vec<HashSet<FileApi>>, SearchFileError> = search_results.into_iter().collect();
    if let Err(e) = res {
        con.close().unwrap();
        log::error!(
            "Failed to search files. Error is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(e);
    }
    let condensed: Vec<HashSet<FileApi>> = res.into_iter().flatten().collect();
    // compare all the files and only include the ones in all the returned lists
    let all_files: HashSet<FileApi> = condensed.iter().cloned().flatten().collect();
    let mut final_set: HashSet<FileApi> = HashSet::new();
    for file in all_files {
        let mut all_match = true;
        for file_set in condensed.iter() {
            if !file_set.iter().find(|f| f.id == file.id).is_some() {
                all_match = false;
                break;
            }
        }
        if all_match {
            final_set.insert(file.clone());
        }
    }
    final_set = final_set.into_iter().unique_by(|f| f.id).collect();
    // now make sure all files have their tags or else we'll get inconsistent response bodies
    let tag_mapping =
        match tag_repository::get_tags_on_files(final_set.iter().map(|f| f.id).collect(), &con) {
            Ok(tags) => tags,
            Err(e) => {
                con.close().unwrap();
                log::error!(
                "Failed to search files - failed to retrieve tags on all files. Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
                return Err(SearchFileError::DbError);
            }
        };
    // using a normal for loop here confuses the rust compiler, and it offers suggestions that just further breaks things.
    // am I doing this wrong? probably...but it works
    let final_set: HashSet<FileApi> = final_set
        .into_iter()
        .map(|mut file| {
            let tags = tag_mapping
                .get(&file.id)
                // not all files here will have tags, especially if the search didn't specify any tags
                .unwrap_or(&Vec::new())
                .iter()
                .cloned()
                .map_into()
                .collect();
            file.tags = tags;
            file
        })
        .collect();
    con.close().unwrap();

    Ok(final_set)
}

fn search_files_by_title(
    search_title: String,
    con: &Connection,
) -> Result<HashSet<FileApi>, SearchFileError> {
    // search text isn't empty
    let searched = match file_repository::search_files(search_title, &con) {
        Ok(f) => f,
        Err(e) => {
            log::error!(
                "Failed to search files by title. Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(SearchFileError::DbError);
        }
    };
    Ok(searched.into_iter().map(|it| it.into()).collect())
}

fn search_files_by_tags(
    search_tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<FileApi>, SearchFileError> {
    let mut matching_files: HashSet<FileApi> = HashSet::new();
    // 1): retrieve all files from the database that have all of the tags directly on them
    let files_with_all_tags: HashSet<FileApi> = match get_files_by_all_tags(search_tags, con) {
        Ok(f) => f,
        Err(e) => {
            log::error!(
                "File search: Failed to retrieve all files by tags. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(SearchFileError::DbError);
        }
    };
    for file in files_with_all_tags {
        matching_files.insert(file);
    }
    // 2): retrieve all folders that have any passed tag
    let folders_with_any_tag = match folder_service::get_folders_by_any_tag(search_tags) {
        Ok(f) => f,
        Err(_) => {
            return Err(SearchFileError::TagError);
        }
    };
    // index of all our folders to make things easier to lookup
    let mut folder_index: HashMap<u32, &FolderResponse> = HashMap::new();
    for folder in folders_with_any_tag.iter() {
        folder_index.insert(folder.id, folder);
    }
    // 3): reduce all the folders to the first folder with all the applied tags
    let reduced = match folder_service::reduce_folders_by_tag(&folders_with_any_tag, search_tags) {
        Ok(folders) => folders,
        Err(_) => {
            log::error!("Failed to search files!\n{}", Backtrace::force_capture());
            return Err(SearchFileError::DbError);
        }
    };
    // 4): get all child files of all reduced folders and their children, because reduced folders have all the tags
    let deduped_child_files: HashSet<FileApi> = match get_deduped_child_files(&reduced, con) {
        Ok(files) => files,
        Err(e) => {
            log::error!(
                "Failed to retrieve deduped child files. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(SearchFileError::DbError);
        }
    };
    // 5: for each folder not in reduced, retrieve all child files in all child folders that contain the remaining tags
    let non_deduped_child_files: HashSet<FileApi> = match get_all_non_reduced_child_files(
        search_tags,
        &folder_index,
        &folders_with_any_tag,
        &reduced,
        con,
    ) {
        Ok(files) => files,
        Err(e) => {
            log::error!(
                "Failed to get child files for non-deduped folders. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(SearchFileError::DbError);
        }
    };
    for file in non_deduped_child_files {
        matching_files.insert(file);
    }
    for file in deduped_child_files {
        matching_files.insert(file);
    }
    Ok(matching_files)
}

fn get_non_duped_folder_ids(
    reduced: &HashSet<FolderResponse>,
    folders_with_any_tag: &HashSet<FolderResponse>,
    con: &Connection,
) -> Result<HashSet<u32>, rusqlite::Error> {
    let non_duped_base_folder_ids: HashSet<u32> = folders_with_any_tag
        .difference(reduced)
        .map(|f| f.id)
        .collect();
    let non_duped_child_folder_ids: HashSet<u32> =
        folder_repository::get_all_child_folder_ids(&non_duped_base_folder_ids, con)?
            .into_iter()
            .collect();
    let non_duped_folder_ids: HashSet<u32> = non_duped_base_folder_ids
        .union(&non_duped_child_folder_ids)
        .copied()
        .collect();
    Ok(non_duped_folder_ids)
}

fn get_files_by_all_tags(
    search_tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<FileApi>, rusqlite::Error> {
    let mut converted_files: HashSet<FileApi> = HashSet::new();
    let files = file_repository::get_files_by_all_tags(search_tags, con)?;
    for file in files {
        let tags: Vec<TagApi> = tag_repository::get_tags_on_file(file.id.unwrap(), con)?
            .into_iter()
            .map(TagApi::from)
            .collect();
        let api = FileApi::from_with_tags(file, tags);
        converted_files.insert(api);
    }
    Ok(converted_files)
}

fn get_deduped_child_files(
    reduced: &HashSet<FolderResponse>,
    con: &Connection,
) -> Result<HashSet<FileApi>, rusqlite::Error> {
    let reduced_ids: Vec<u32> = reduced.iter().map(|f| f.id).collect();
    let all_relevant_folder_ids: HashSet<u32> =
        folder_repository::get_all_child_folder_ids(&reduced_ids, con)?
            .into_iter()
            .chain(reduced_ids)
            .collect();
    let deduped_child_files = get_child_files(&all_relevant_folder_ids, con)?;
    Ok(deduped_child_files)
}

fn get_child_files(
    ids: &HashSet<u32>,
    con: &Connection,
) -> Result<HashSet<FileApi>, rusqlite::Error> {
    let files: HashSet<FileRecord> = if ids.is_empty() {
        HashSet::new()
    } else {
        folder_repository::get_child_files(ids.clone(), con)?
            .into_iter()
            .collect()
    };
    let mut converted: HashSet<FileApi> = HashSet::new();
    for file in files {
        let tags = tag_repository::get_tags_on_file(file.id.unwrap(), con)?
            .into_iter()
            .map(TagApi::from)
            .collect();
        converted.insert(FileApi::from_with_tags(file, tags));
    }
    Ok(converted)
}

/// recursively retrieves all child files with the remaining tags in `search_tags` for each folder in `folders_with_any_tag`
fn get_all_non_reduced_child_files(
    search_tags: &HashSet<String>,
    folder_index: &HashMap<u32, &FolderResponse>,
    folders_with_any_tag: &HashSet<FolderResponse>,
    reduced: &HashSet<FolderResponse>,
    con: &Connection,
) -> Result<HashSet<FileApi>, rusqlite::Error> {
    let non_duped_folder_ids: HashSet<u32> =
        get_non_duped_folder_ids(reduced, folders_with_any_tag, con)?;
    // 5.1) retrieve all child files of all child folders (+ original folder) using method in #4.2 above
    let remaining_child_files: HashSet<FileApi> = get_child_files(&non_duped_folder_ids, con)?
        .into_iter()
        .collect();
    let mut final_files: HashSet<FileApi> = HashSet::new();
    for file in remaining_child_files {
        // TODO the file might not have a direct parent folder in the index, but could still have an ancestor folder in the index
        // TODO is_ancestor_of method in repository layer, that takes a folder id and file id, and returns true if folder is an ancestor of the file
        let parent_id = file.folder_id.unwrap_or_default();
        let parent_tags: HashSet<String> = if let Some(parent_folder) = folder_index.get(&parent_id)
        {
            parent_folder
                .tags
                .iter()
                .map(|it| it.title.clone())
                .collect()
        } else {
            // direct parent isn't in the index, meaning this file has a searched tag but a grandparent has other searched tags. We need to find which parent that was and return those tags
            let parent_ids = folder_index.keys();
            let mut all_ancestor_tags: HashSet<String> = HashSet::new();
            for parent_id in parent_ids {
                if is_ancestor_of(*parent_id, &file, con)? {
                    let tag_titles = folder_index
                        .get(parent_id)
                        .expect("parent id somehow disappeared from map")
                        .tags
                        .iter()
                        .map(|it| it.title.clone());
                    all_ancestor_tags.extend(tag_titles);
                }
            }
            all_ancestor_tags
        };
        // parent folder has all the tags, we don't need to check further files
        if &parent_tags == search_tags {
            continue;
        }
        let missing_tags: HashSet<&String> = search_tags.difference(&parent_tags).collect();
        let file_tags: HashSet<&String> = file.tags.iter().map(|tag| &tag.title).collect();
        if missing_tags == file_tags {
            final_files.insert(file);
        }
    }
    Ok(final_files)
}

/// Checks if the passed `potential_ancestor_id` is a parent/grandparent/great grandparent/etc of the passed `file`
/// If the file has no parent id or `potential_ancestor_id` == `file.folder_id`, no database call is made. Otherwise, the database is
/// checked to see if `potential_ancestor_id` is an ancestor.
///
/// This function does not close the connection passed to it.
fn is_ancestor_of(
    potential_ancestor_id: u32,
    file: &FileApi,
    con: &Connection,
) -> Result<bool, rusqlite::Error> {
    return if let Some(direct_parent_id) = file.folder_id {
        // avoid having to make a db call if the potential ancestor is a direct parent
        if direct_parent_id == potential_ancestor_id {
            Ok(true)
        } else {
            match folder_repository::get_ancestor_folder_ids(direct_parent_id, con) {
                Ok(parent_ids) => Ok(parent_ids.contains(&potential_ancestor_id)),
                Err(e) => {
                    log::error!(
                        "Failed to get ancestor folder ids. Exception is {e}\n{}",
                        Backtrace::force_capture()
                    );
                    Err(e)
                }
            }
        }
    } else {
        // file is at root, so no folder will ever be its parent unless the ancestor id is also root
        return Ok(potential_ancestor_id == 0);
    };
}

#[cfg(test)]
mod search_files_tests {
    use std::collections::HashSet;

    use chrono::NaiveDateTime;

    use crate::model::api::{FileApi, FileTypes};
    use crate::model::response::TagApi;
    use crate::service::search_service::search_files;
    use crate::test::{
        cleanup, create_file_db_entry, create_folder_db_entry, create_tag_file, create_tag_files,
        create_tag_folder, create_tag_folders, refresh_db,
    };

    #[test]
    fn search_files_works() {
        refresh_db();
        create_file_db_entry("test", None);
        create_file_db_entry("test2", None);
        let res = search_files("test2".to_string(), vec![])
            .unwrap()
            .into_iter()
            .collect::<Vec<FileApi>>();
        assert_eq!(1, res.len());
        let res = &res[0];
        assert_eq!(res.id, 2);
        assert_eq!(res.name, "test2".to_string());
        assert_eq!(res.folder_id, None);
        assert_eq!(res.tags, vec![]);
        assert_eq!(res.file_type, Some(FileTypes::Unknown));
        assert_eq!(res.size, Some(0));
        cleanup();
    }

    #[test]
    fn search_files_includes_file_tags() {
        refresh_db();
        create_file_db_entry("first", None);
        create_file_db_entry("second", None);
        create_tag_file("tag1", 1);
        create_tag_files("tag", vec![1, 2]);
        let res = search_files("".to_string(), vec!["tag1".to_string(), "tag".to_string()])
            .unwrap()
            .into_iter()
            .collect::<Vec<FileApi>>();
        assert_eq!(1, res.len());
        let res = &res[0];
        assert_eq!(res.id, 1);
        assert_eq!(res.name, "first".to_string());
        assert_eq!(res.folder_id, None);
        assert_eq!(
            res.tags,
            vec![
                TagApi {
                    id: Some(1),
                    title: "tag1".to_string(),
                },
                TagApi {
                    id: Some(2),
                    title: "tag".to_string(),
                }
            ]
        );
        assert_eq!(res.file_type, Some(FileTypes::Unknown));
        assert_eq!(res.size, Some(0));
        cleanup();
    }

    #[test]
    fn search_files_tags_and_title() {
        refresh_db();
        create_file_db_entry("first", None);
        create_file_db_entry("second", None);
        create_tag_files("tag", vec![1, 2]);
        let res = search_files("first".to_string(), vec!["tag".to_string()])
            .unwrap()
            .into_iter()
            .collect::<Vec<FileApi>>();
        assert_eq!(1, res.len());
        let res = &res[0];
        assert_eq!(res.id, 1);
        assert_eq!(res.name, "first".to_string());
        assert_eq!(res.folder_id, None);
        assert_eq!(
            res.tags,
            vec![TagApi {
                id: Some(1),
                title: "tag".to_string(),
            }]
        );
        assert_eq!(res.file_type, Some(FileTypes::Unknown));
        assert_eq!(res.size, Some(0));
        cleanup();
    }

    #[test]
    fn search_files_includes_parent_folder_tags() {
        refresh_db();
        create_folder_db_entry("top", None); // 1
        create_folder_db_entry("middle", Some(1)); // 2
        create_folder_db_entry("bottom", Some(2)); // 3
        create_file_db_entry("top file", Some(1));
        create_file_db_entry("bottom file", Some(3));
        create_tag_folders("tag1", vec![1, 3]); // tag1 on top folder and bottom folder
        create_tag_folder("tag2", 3); // tag2 only on bottom folder
                                      // tag1 should retrieve all files
        let res = search_files("".to_string(), vec!["tag1".to_string()]).unwrap();
        // we have to convert res to a vec in order to not care about the create date, since hash set `contains` relies on hash
        let res: Vec<FileApi> = res.iter().cloned().collect();
        assert_eq!(2, res.len());
        assert!(res.contains(&FileApi {
            id: 1,
            name: "top file".to_string(),
            folder_id: Some(1),
            tags: vec![],
            size: Some(0),
            create_date: None,
            file_type: Some(FileTypes::Unknown)
        }));
        assert!(res.contains(&FileApi {
            id: 2,
            name: "bottom file".to_string(),
            folder_id: Some(3),
            tags: vec![],
            size: Some(0),
            create_date: None,
            file_type: Some(FileTypes::Unknown)
        }));
        let res = search_files("".to_string(), vec!["tag2".to_string()]).unwrap();
        let res: Vec<FileApi> = res.iter().cloned().collect();
        assert!(res.contains(&FileApi {
            id: 2,
            name: "bottom file".to_string(),
            folder_id: Some(3),
            tags: vec![],
            size: Some(0),
            create_date: None,
            file_type: Some(FileTypes::Unknown)
        }));
        cleanup();
    }

    #[test]
    fn search_files_handles_partial_tag_folders() {
        refresh_db();
        create_folder_db_entry("top", None);
        create_file_db_entry("good", Some(1));
        create_file_db_entry("bad", Some(1));
        create_tag_folders("tag1", vec![1]);
        create_tag_file("tag2", 1);
        let res: HashSet<String> =
            search_files(String::new(), vec!["tag1".to_string(), "tag2".to_string()])
                .unwrap()
                .into_iter()
                .map(|it| it.name)
                .collect();
        assert_eq!(HashSet::from(["good".to_string()]), res);
        cleanup();
    }

    #[test]
    fn search_files_handles_folder_tag_and_file_tag_with_folder_separate() {
        refresh_db();
        create_folder_db_entry("top", None); // 1
        create_folder_db_entry("middle", Some(1)); // 2
        create_tag_folder("top", 1);
        let good_file = FileApi {
            id: 1,
            folder_id: Some(2),
            name: "good".to_string(),
            tags: vec![TagApi {
                id: None,
                title: "file".to_string(),
            }],
            size: Some(0),
            create_date: Some(NaiveDateTime::default()),
            file_type: Some(FileTypes::Unknown),
        }
        .save_to_db();
        FileApi {
            id: 2,
            folder_id: Some(2),
            name: "bad".to_string(),
            tags: vec![TagApi {
                id: None,
                title: "something_else".to_string(),
            }],
            size: None,
            create_date: None,
            file_type: None,
        }
        .save_to_db();
        let res = search_files(String::new(), vec!["top".to_string(), "file".to_string()]).unwrap();
        let expected = HashSet::from_iter(vec![good_file].into_iter());
        assert_eq!(expected, res);
        cleanup();
    }
}
