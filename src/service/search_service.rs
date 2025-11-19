use std::backtrace::Backtrace;
use std::collections::HashSet;

use itertools::Itertools;
use rusqlite::Connection;

use crate::model::api::FileApi;
use crate::model::error::file_errors::SearchFileError;
use crate::model::request::attributes::AttributeSearch;
use crate::repository::{file_repository, open_connection};
use crate::tags::repository as tag_repository;

pub fn search_files(
    search_title: &str,
    search_tags: Vec<String>,
    search_attributes: AttributeSearch,
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
    if !search_attributes.is_empty() {
        search_results.push(search_files_by_attributes(search_attributes, &con));
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
    let all_files: HashSet<FileApi> = condensed.iter().flatten().cloned().collect();
    let mut final_set: HashSet<FileApi> = HashSet::new();
    for file in all_files {
        let mut all_match = true;
        for file_set in condensed.iter() {
            if !file_set.iter().any(|f| f.id == file.id) {
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
    let tag_mapping = match tag_repository::get_tags_on_files(
        final_set.iter().map(|f| f.id).collect(),
        &con,
    ) {
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
    search_title: &str,
    con: &Connection,
) -> Result<HashSet<FileApi>, SearchFileError> {
    // search text isn't empty
    file_repository::search_files(search_title, con)
        .map(|it| it.into_iter().map(FileApi::from).collect())
        .map_err(|e| {
            log::error!(
                "Failed to search files by title. Error is {e:?}\n{}",
                Backtrace::force_capture()
            );
            SearchFileError::DbError
        })
}

fn search_files_by_tags(
    search_tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<FileApi>, SearchFileError> {
    let retrieved = file_repository::search_files_by_tags(search_tags, con);
    let matching_files = match retrieved {
        Ok(f) => f,
        Err(e) => {
            log::error!(
                "Failed to search files by tags: {e:?}\n{:?}",
                Backtrace::force_capture()
            );
            return Err(SearchFileError::DbError);
        }
    };
    Ok(matching_files.into_iter().map(|it| it.into()).collect())
}

fn search_files_by_attributes(
    attributes: AttributeSearch,
    con: &Connection,
) -> Result<HashSet<FileApi>, SearchFileError> {
    file_repository::search_files_by_attributes(attributes, con)
        .map(|it| it.into_iter().map(FileApi::from).collect())
        .map_err(|it| {
            log::error!(
                "failed to search file attributes; {it:?}\n{}",
                Backtrace::force_capture()
            );
            SearchFileError::DbError
        })
}

#[cfg(test)]
mod search_files_tests {
    use std::collections::HashSet;

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use super::search_files;
    use crate::model::api::FileApi;
    use crate::model::file_types::FileTypes;
    use crate::model::repository::FileRecord;
    use crate::model::request::attributes::{
        AttributeSearch, AttributeTypes, EqualityOperator, NamedAttributes,
        NamedComparisonAttribute,
    };
    use crate::model::response::TaggedItemApi;
    use crate::test::{
        cleanup, create_file_db_entry, create_folder_db_entry, create_tag_file, create_tag_files,
        create_tag_folder, create_tag_folders, imply_tag_on_file, init_db_folder,
    };

    #[test]
    fn search_files_works() {
        init_db_folder();
        create_file_db_entry("test", None);
        create_file_db_entry("test2", None);
        let res = search_files("test2", vec![], vec![].try_into().unwrap())
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
        init_db_folder();
        create_file_db_entry("first", None);
        create_file_db_entry("second", None);
        create_tag_file("tag1", 1);
        create_tag_files("tag", vec![1, 2]);
        let res = search_files(
            "",
            vec!["tag1".to_string(), "tag".to_string()],
            vec![].try_into().unwrap(),
        )
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
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "tag1".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: Some(2),
                    title: "tag".to_string(),
                    implicit_from: None,
                }
            ]
        );
        assert_eq!(res.file_type, Some(FileTypes::Unknown));
        assert_eq!(res.size, Some(0));
        cleanup();
    }

    #[test]
    fn search_files_tags_and_title() {
        init_db_folder();
        create_file_db_entry("first", None);
        create_file_db_entry("second", None);
        create_tag_files("tag", vec![1, 2]);
        let res = search_files("first", vec!["tag".to_string()], vec![].try_into().unwrap())
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
            vec![TaggedItemApi {
                tag_id: Some(1),
                title: "tag".to_string(),
                implicit_from: None,
            }]
        );
        assert_eq!(res.file_type, Some(FileTypes::Unknown));
        assert_eq!(res.size, Some(0));
        cleanup();
    }

    #[test]
    fn search_files_includes_parent_folder_tags() {
        init_db_folder();
        create_folder_db_entry("top", None); // 1
        create_folder_db_entry("middle", Some(1)); // 2
        create_folder_db_entry("bottom", Some(2)); // 3
        create_file_db_entry("top file", Some(1));
        create_file_db_entry("bottom file", Some(3));
        create_tag_folders("tag1", vec![1, 3]); // tag1 on top folder and bottom folder
        create_tag_folder("tag2", 3); // tag2 only on bottom folder
        imply_tag_on_file(1, 1, 1);
        imply_tag_on_file(1, 2, 3);
        imply_tag_on_file(2, 2, 3);
        // tag1 should retrieve all files
        let res = search_files("", vec!["tag1".to_string()], vec![].try_into().unwrap()).unwrap();
        // we have to convert res to a vec in order to not care about the create date, since hash set `contains` relies on hash
        let res: Vec<FileApi> = res.iter().cloned().collect();
        log::debug!("first round: {res:?}");
        assert_eq!(2, res.len());
        assert!(res.contains(&FileApi {
            id: 1,
            name: "top file".to_string(),
            folder_id: Some(1),
            tags: vec![TaggedItemApi {
                tag_id: Some(1),
                title: "tag1".to_string(),
                implicit_from: Some(1)
            }],
            size: Some(0),
            date_created: None,
            file_type: Some(FileTypes::Unknown)
        }));
        assert!(res.contains(&FileApi {
            id: 2,
            name: "bottom file".to_string(),
            folder_id: Some(3),
            tags: vec![
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "tag1".to_string(),
                    implicit_from: Some(3)
                },
                TaggedItemApi {
                    tag_id: Some(2),
                    title: "tag2".to_string(),
                    implicit_from: Some(3)
                }
            ],
            size: Some(0),
            date_created: None,
            file_type: Some(FileTypes::Unknown)
        }));
        let res = search_files("", vec!["tag2".to_string()], vec![].try_into().unwrap()).unwrap();
        let res: Vec<FileApi> = res.iter().cloned().collect();
        log::debug!("{res:?}");
        assert!(res.contains(&FileApi {
            id: 2,
            name: "bottom file".to_string(),
            folder_id: Some(3),
            tags: vec![
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "tag1".to_string(),
                    implicit_from: Some(3)
                },
                TaggedItemApi {
                    tag_id: Some(2),
                    title: "tag2".to_string(),
                    implicit_from: Some(3)
                }
            ],
            size: Some(0),
            date_created: None,
            file_type: Some(FileTypes::Unknown)
        }));
        cleanup();
    }

    #[test]
    fn search_files_handles_partial_tag_folders() {
        init_db_folder();
        create_folder_db_entry("top", None);
        create_file_db_entry("good", Some(1));
        create_file_db_entry("bad", Some(1));
        create_tag_folders("tag1", vec![1]);
        create_tag_file("tag2", 1);
        imply_tag_on_file(1, 1, 1);
        let res: HashSet<String> = search_files(
            "",
            vec!["tag1".to_string(), "tag2".to_string()],
            vec![].try_into().unwrap(),
        )
        .unwrap()
        .into_iter()
        .map(|it| it.name)
        .collect();
        assert_eq!(HashSet::from(["good".to_string()]), res);
        cleanup();
    }

    #[test]
    fn search_files_handles_folder_tag_and_file_tag_with_folder_separate() {
        init_db_folder();
        create_folder_db_entry("top", None); // 1
        create_folder_db_entry("middle", Some(1)); // 2
        create_tag_folder("top", 1);
        let good_file = FileApi {
            id: 1,
            folder_id: Some(2),
            name: "good".to_string(),
            tags: vec![TaggedItemApi {
                tag_id: Some(1),
                title: "file".to_string(),
                implicit_from: Some(1),
            }],
            size: Some(0),
            date_created: Some(NaiveDateTime::default()),
            file_type: Some(FileTypes::Unknown),
        }
        .save_to_db();
        imply_tag_on_file(1, 1, 1);
        FileApi {
            id: 2,
            folder_id: Some(2),
            name: "bad".to_string(),
            tags: vec![TaggedItemApi {
                tag_id: None,
                title: "something_else".to_string(),
                implicit_from: None,
            }],
            size: None,
            date_created: None,
            file_type: None,
        }
        .save_to_db();
        let res: HashSet<u32> = search_files(
            "",
            vec!["top".to_string(), "file".to_string()],
            vec![].try_into().unwrap(),
        )
        .map(|it| it.iter().map(|i| i.id).collect())
        .unwrap();
        let expected: HashSet<u32> = HashSet::from_iter(vec![good_file.id]);
        assert_eq!(expected, res);
        cleanup();
    }

    #[test]
    fn search_attributes() {
        init_db_folder();
        let day = NaiveDate::from_ymd_opt(2022, 8, 26).unwrap();
        let time = NaiveTime::from_hms_opt(21, 48, 00).unwrap();
        let good = FileRecord {
            id: None,
            name: "test".to_string(),
            parent_id: None,
            create_date: NaiveDateTime::new(day, time),
            size: 9087239875,
            file_type: FileTypes::Unknown,
        }
        .save_to_db();
        FileRecord {
            id: None,
            name: "bad".to_string(),
            parent_id: None,
            create_date: crate::test::now(),
            size: 0,
            file_type: FileTypes::Application,
        }
        .save_to_db();
        let attributes = AttributeSearch {
            attributes: vec![AttributeTypes::Named(NamedComparisonAttribute {
                field: NamedAttributes::FileType,
                value: "unknown".to_string(),
                operator: EqualityOperator::Eq,
            })],
        };
        let expected: HashSet<FileApi> = [good].into_iter().map(FileApi::from).collect();
        let actual = search_files("", vec![], attributes);
        assert_eq!(Ok(expected), actual);
        cleanup();
    }

    #[test]
    fn search_title_and_attributes() {
        init_db_folder();
        let day = NaiveDate::from_ymd_opt(2022, 8, 26).unwrap();
        let time = NaiveTime::from_hms_opt(21, 48, 00).unwrap();
        let good = FileRecord {
            id: None,
            name: "good".to_string(),
            parent_id: None,
            create_date: NaiveDateTime::new(day, time),
            size: 9087239875,
            file_type: FileTypes::Unknown,
        }
        .save_to_db();
        FileRecord {
            id: None,
            name: "bad".to_string(),
            parent_id: None,
            create_date: crate::test::now(),
            size: 0,
            file_type: FileTypes::Unknown,
        }
        .save_to_db();
        let attributes = AttributeSearch {
            attributes: vec![AttributeTypes::Named(NamedComparisonAttribute {
                field: NamedAttributes::FileType,
                value: "unknown".to_string(),
                operator: EqualityOperator::Eq,
            })],
        };
        let expected: HashSet<FileApi> = [good].into_iter().map(FileApi::from).collect();
        let actual = search_files("good", vec![], attributes);
        assert_eq!(Ok(expected), actual);
        cleanup();
    }

    #[test]
    fn search_tags_and_attributes() {
        init_db_folder();
        let day = NaiveDate::from_ymd_opt(2022, 8, 26).unwrap();
        let time = NaiveTime::from_hms_opt(21, 48, 00).unwrap();
        FileRecord {
            id: None,
            name: "good".to_string(),
            parent_id: None,
            create_date: NaiveDateTime::new(day, time),
            size: 9087239875,
            file_type: FileTypes::Unknown,
        }
        .save_to_db();
        create_tag_file("good", 1);
        FileRecord {
            id: None,
            name: "bad".to_string(),
            parent_id: None,
            create_date: crate::test::now(),
            size: 0,
            file_type: FileTypes::Unknown,
        }
        .save_to_db();
        create_tag_file("bad", 2);
        let attributes = AttributeSearch {
            attributes: vec![AttributeTypes::Named(NamedComparisonAttribute {
                field: NamedAttributes::FileType,
                value: "unknown".to_string(),
                operator: EqualityOperator::Eq,
            })],
        };
        let actual: Vec<FileApi> = search_files("good", vec![], attributes)
            .unwrap()
            .into_iter()
            .collect();
        assert_eq!(1, actual[0].id);
        assert_eq!(1, actual.len());
        cleanup();
    }
}
