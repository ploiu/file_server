use std::collections::{HashMap, HashSet};

use regex::RegexBuilder;
use rusqlite::{Connection, Error};

use crate::model::api::FileApi;
use crate::model::error::file_errors::SearchFileError;
use crate::model::repository::FileRecord;
use crate::model::response::folder_responses::FolderResponse;
use crate::model::response::TagApi;
use crate::repository::{file_repository, folder_repository, open_connection, tag_repository};
use crate::service::folder_service;

pub fn search_files(
    search_title: String,
    search_tags: Vec<String>,
) -> Result<HashSet<FileApi>, SearchFileError> {
    let search_tags: HashSet<String> = HashSet::from_iter(search_tags);
    let con: Connection = open_connection();
    let mut matching_files: HashSet<FileApi> = HashSet::new();
    if !search_tags.is_empty() {
        let tag_search_files: HashSet<FileApi> = match search_files_by_tags(&search_tags, &con) {
            Ok(files) => files,
            Err(e) => {
                con.close().unwrap();
                log::error!("Failed to search files by tags. Exception is {e}");
                return Err(SearchFileError::DbError);
            }
        };
        for file in tag_search_files {
            matching_files.insert(file);
        }
        if !search_title.is_empty() {
            let search_gex = RegexBuilder::new(search_title.as_str())
                .case_insensitive(true)
                .build()
                .expect(format!("Failed to build regex for search term [{search_title}]").as_str());
            matching_files = matching_files
                .iter()
                .filter(|file| search_gex.is_match(file.name.as_str()))
                .map(|file| file.clone())
                .collect();
        }
    } else {
        // search text isn't empty
        let searched = match file_repository::search_files(search_title, &con) {
            Ok(f) => f,
            Err(e) => {
                con.close().unwrap();
                log::error!("Failed to search files. Exception is {e}");
                return Err(SearchFileError::DbError);
            }
        };
        for file in searched {
            matching_files.insert(FileApi {
                id: file.id.unwrap(),
                folder_id: file.parent_id,
                name: file.name.clone(),
                tags: Vec::new(),
            });
        }
    }
    con.close().unwrap();
    Ok(matching_files)
}

fn search_files_by_tags(
    search_tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<FileApi>, SearchFileError> {
    let mut matching_files: HashSet<FileApi> = HashSet::new();
    // 1): retrieve all files from the database that have all of the tags directly on them
    let files_with_all_tags: HashSet<FileApi> = match get_files_by_all_tags(&search_tags, &con) {
        Ok(f) => f,
        Err(e) => {
            log::error!("File search: Failed to retrieve all files by tags. Exception is {e}");
            return Err(SearchFileError::DbError);
        }
    };
    for file in files_with_all_tags {
        matching_files.insert(file);
    }
    // 2): retrieve all folders that have any passed tag
    let folders_with_any_tag = match folder_service::get_folders_by_any_tag(&search_tags) {
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
    let reduced = match folder_service::reduce_folders_by_tag(&folders_with_any_tag, &search_tags) {
        Ok(folders) => folders,
        Err(_) => {
            log::error!("Failed to search files!");
            return Err(SearchFileError::DbError);
        }
    };
    // 4): get all child files of all deduped folders and their children
    let deduped_child_files: HashSet<FileApi> = match get_deduped_child_files(&reduced, &con) {
        Ok(files) => files,
        Err(e) => {
            log::error!("Failed to retrieve deduped child files. Exception is {e}");
            return Err(SearchFileError::DbError);
        }
    };
    // 5: for each folder not deduplicated, retrieve all child files in all child folders that contain the remaining tags
    let non_deduped_child_files: HashSet<FileApi> = match get_all_non_deduped_child_files(
        &search_tags,
        &folder_index,
        &folders_with_any_tag,
        &reduced,
        &con,
    ) {
        Ok(files) => files,
        Err(e) => {
            log::error!("Failed to get child files for non-deduped folders. Exception is {e}");
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
        .map(|it| it.clone())
        .collect();
    Ok(non_duped_folder_ids)
}

fn get_files_by_all_tags(
    search_tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<FileApi>, rusqlite::Error> {
    let mut converted_files: HashSet<FileApi> = HashSet::new();
    let mut files = file_repository::get_files_by_all_tags(search_tags, con)?;
    for file in files {
        let tags: Vec<TagApi> = tag_repository::get_tags_on_file(file.id.unwrap(), con)?
            .into_iter()
            .map(TagApi::from)
            .collect();
        converted_files.insert(FileApi {
            id: file.id.unwrap(),
            folder_id: file.parent_id,
            name: file.name.clone(),
            tags,
        });
    }
    Ok(converted_files)
}

fn get_deduped_child_files(
    reduced: &HashSet<FolderResponse>,
    con: &Connection,
) -> Result<HashSet<FileApi>, rusqlite::Error> {
    let reduced_ids: Vec<u32> = reduced.iter().map(|f| f.id.clone()).collect();
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
        converted.insert(FileApi {
            id: file.id.unwrap_or(0),
            name: file.name.clone(),
            folder_id: file.parent_id,
            tags,
        });
    }
    Ok(converted)
}

fn get_all_non_deduped_child_files(
    search_tags: &HashSet<String>,
    folder_index: &HashMap<u32, &FolderResponse>,
    folders_with_any_tag: &HashSet<FolderResponse>,
    reduced: &HashSet<FolderResponse>,
    con: &Connection,
) -> Result<HashSet<FileApi>, rusqlite::Error> {
    let non_duped_folder_ids: HashSet<u32> =
        get_non_duped_folder_ids(&reduced, &folders_with_any_tag, &con)?;
    // 5.1) retrieve all child files of all child folders (+ original folder) using method in #4.2 above
    let remaining_child_files: HashSet<FileApi> = get_child_files(&non_duped_folder_ids, &con)?
        .into_iter()
        .collect();
    let mut final_files: HashSet<FileApi> = HashSet::new();
    for file in remaining_child_files {
        let parent_id = file.folder_id.unwrap_or(0);
        let parent_tags: HashSet<String> = folder_index
            .get(&parent_id)
            .unwrap()
            .tags
            .iter()
            .map(|it| it.title.clone())
            .collect();
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

#[cfg(test)]
mod search_files_tests {
    use std::collections::HashSet;

    use crate::model::api::FileApi;
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
        assert_eq!(
            vec![FileApi {
                id: 2,
                name: "test2".to_string(),
                folder_id: None,
                tags: vec![],
            }],
            res
        );
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
        // should only return the first one since it has both tags
        assert_eq!(
            vec![FileApi {
                id: 1,
                name: "first".to_string(),
                folder_id: None,
                tags: vec![
                    TagApi {
                        id: Some(1),
                        title: "tag1".to_string(),
                    },
                    TagApi {
                        id: Some(2),
                        title: "tag".to_string(),
                    },
                ],
            }],
            res
        );
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
        assert_eq!(
            vec![FileApi {
                id: 1,
                name: "first".to_string(),
                folder_id: None,
                tags: vec![TagApi {
                    id: Some(1),
                    title: "tag".to_string(),
                }],
            }],
            res
        );
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
        assert_eq!(
            HashSet::from([
                FileApi {
                    id: 1,
                    name: "top file".to_string(),
                    folder_id: Some(1),
                    tags: vec![],
                },
                FileApi {
                    id: 2,
                    name: "bottom file".to_string(),
                    folder_id: Some(3),
                    tags: vec![],
                }
            ]),
            res
        );
        let res = search_files("".to_string(), vec!["tag2".to_string()]).unwrap();
        assert_eq!(
            HashSet::from([FileApi {
                id: 2,
                name: "bottom file".to_string(),
                folder_id: Some(3),
                tags: vec![],
            }]),
            res
        );
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
}
