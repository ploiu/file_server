mod create_tag_tests {
    use crate::repository::open_connection;
    use crate::tags::Tag;
    use crate::tags::repository;
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn create_tag() {
        init_db_folder();
        let con = open_connection();
        let tag = repository::create_tag("test", &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test".to_string(),
            },
            tag
        );
        cleanup();
    }
}

mod get_tag_by_title_tests {
    use crate::repository::open_connection;
    use crate::tags::Tag;
    use crate::tags::repository::{create_tag, get_tag_by_title};
    use crate::test::*;

    #[test]
    fn get_tag_by_title_found() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        let found = get_tag_by_title("TeSt", &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Some(Tag {
                id: 1,
                title: "test".to_string(),
            }),
            found
        );
        cleanup();
    }
    #[test]
    fn get_tag_by_title_not_found() {
        init_db_folder();
        let con = open_connection();
        let not_found = get_tag_by_title("test", &con).unwrap();
        con.close().unwrap();
        assert_eq!(None, not_found);
        cleanup();
    }
}

mod get_tag_by_id_tests {
    use crate::repository::open_connection;
    use crate::tags::Tag;
    use crate::tags::repository::{create_tag, get_tag};
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn get_tag_success() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        let tag = get_tag(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test".to_string(),
            },
            tag
        );
        cleanup();
    }
}

mod update_tag_tests {
    use crate::repository::open_connection;
    use crate::tags::Tag;
    use crate::tags::repository::{create_tag, get_tag, update_tag};
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn update_tag_success() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        update_tag(
            Tag {
                id: 1,
                title: "test2".to_string(),
            },
            &con,
        )
        .unwrap();
        let res = get_tag(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test2".to_string(),
            },
            res
        );
        cleanup();
    }
}

mod delete_tag_tests {
    use crate::repository::open_connection;
    use crate::tags::repository::{create_tag, delete_tag, get_tag};
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn delete_tag_success() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        delete_tag(1, &con).unwrap();
        let not_found = get_tag(1, &con);
        con.close().unwrap();
        assert_eq!(Err(rusqlite::Error::QueryReturnedNoRows), not_found);
        cleanup();
    }
}

mod get_tag_on_file_tests {
    use crate::model::file_types::FileTypes;
    use crate::model::repository::FileRecord;
    use crate::repository::file_repository::create_file;
    use crate::repository::open_connection;
    use crate::tags::TaggedItem;
    use crate::tags::repository::*;
    use crate::test::*;

    #[test]
    fn get_tags_on_file_returns_tags() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        create_tag("test2", &con).unwrap();
        create_file(
            &FileRecord {
                id: None,
                name: "test_file".to_string(),
                parent_id: None,
                create_date: now(),
                size: 0,
                file_type: FileTypes::Unknown,
            },
            &con,
        )
        .unwrap();
        add_explicit_tag_to_file(1, 1, &con).unwrap();
        add_explicit_tag_to_file(1, 2, &con).unwrap();
        let res = get_all_tags_for_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            vec![
                TaggedItem {
                    id: 1,
                    tag_id: 1,
                    title: "test".to_string(),
                    file_id: Some(1),
                    folder_id: None,
                    implicit_from_id: None
                },
                TaggedItem {
                    id: 2,
                    tag_id: 2,
                    title: "test2".to_string(),
                    file_id: Some(1),
                    folder_id: None,
                    implicit_from_id: None
                }
            ],
            res
        );
        cleanup();
    }
    #[test]
    fn get_tags_on_file_returns_nothing_if_no_tags() {
        init_db_folder();
        let con = open_connection();
        create_file(
            &FileRecord {
                id: None,
                name: "test_file".to_string(),
                parent_id: None,
                create_date: now(),
                size: 0,
                file_type: FileTypes::Application,
            },
            &con,
        )
        .unwrap();
        let res = get_all_tags_for_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<TaggedItem>::new(), res);
        cleanup();
    }
}

mod remove_tag_from_file_tests {
    use crate::model::file_types::FileTypes;
    use crate::model::repository::FileRecord;
    use crate::repository::file_repository::create_file;
    use crate::repository::open_connection;
    use crate::tags::TaggedItem;
    use crate::tags::repository::*;
    use crate::test::{cleanup, init_db_folder, now};

    #[test]
    fn remove_tag_from_file_works() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        create_file(
            &FileRecord {
                id: None,
                name: "test_file".to_string(),
                parent_id: None,
                create_date: now(),
                size: 0,
                file_type: FileTypes::Unknown,
            },
            &con,
        )
        .unwrap();
        remove_explicit_tag_from_file(1, 1, &con).unwrap();
        let tags = get_all_tags_for_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<TaggedItem>::new(), tags);
        cleanup();
    }
}

mod get_tag_on_folder_tests {
    use crate::model::repository::Folder;
    use crate::repository::folder_repository::create_folder;
    use crate::repository::open_connection;
    use crate::tags::TaggedItem;
    use crate::tags::repository::{
        add_explicit_tag_to_folder, create_tag, get_all_tags_for_folder,
    };
    use crate::test::*;

    #[test]
    fn get_tags_on_folder_returns_tags() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        create_tag("test2", &con).unwrap();
        create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        add_explicit_tag_to_folder(1, 1, &con).unwrap();
        add_explicit_tag_to_folder(1, 2, &con).unwrap();
        let res = get_all_tags_for_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            vec![
                TaggedItem {
                    id: 1,
                    tag_id: 1,
                    title: "test".to_string(),
                    folder_id: Some(1),
                    file_id: None,
                    implicit_from_id: None
                },
                TaggedItem {
                    id: 2,
                    tag_id: 2,
                    title: "test2".to_string(),
                    folder_id: Some(1),
                    file_id: None,
                    implicit_from_id: None
                }
            ],
            res
        );
        cleanup();
    }
    #[test]
    fn get_tags_on_folder_returns_nothing_if_no_tags() {
        init_db_folder();
        let con = open_connection();
        create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        let res = get_all_tags_for_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<TaggedItem>::new(), res);
        cleanup();
    }
}

mod remove_tag_from_folder_tests {
    use crate::model::repository::Folder;
    use crate::repository::folder_repository::create_folder;
    use crate::repository::open_connection;
    use crate::tags::TaggedItem;
    use crate::tags::repository::{
        create_tag, get_all_tags_for_folder, remove_explicit_tag_from_folder,
    };
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn remove_tag_from_folder_works() {
        init_db_folder();
        let con = open_connection();
        create_tag("test", &con).unwrap();
        create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        remove_explicit_tag_from_folder(1, 1, &con).unwrap();
        let tags = get_all_tags_for_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<TaggedItem>::new(), tags);
        cleanup();
    }
}

mod get_tags_on_files_tests {
    use std::collections::HashMap;

    use crate::tags::TaggedItem;
    use crate::tags::repository::get_all_tags_for_files;
    use crate::{repository::open_connection, test::*};

    #[test]
    fn returns_proper_mapping_for_file_tags() {
        init_db_folder();
        create_file_db_entry("file1", None);
        create_file_db_entry("file2", None);
        create_file_db_entry("control", None);
        create_tag_file("tag1", 1);
        create_tag_file("tag2", 1);
        create_tag_file("tag3", 2);
        let con = open_connection();
        let res = get_all_tags_for_files(vec![1, 2, 3], &con).unwrap();
        con.close().unwrap();
        #[rustfmt::skip]
        let expected = HashMap::from([
            (1, vec![
                    TaggedItem {id: 1, tag_id: 1, file_id: Some(1), folder_id: None, title: "tag1".to_string(), implicit_from_id: None}, 
                    TaggedItem {id: 2, tag_id: 2, file_id: Some(1), folder_id: None, title: "tag2".to_string(), implicit_from_id: None},
                ]
            ),
            (2, vec![TaggedItem {id: 3, tag_id: 3, file_id: Some(2), folder_id: None, title: "tag3".to_string(), implicit_from_id: None}])
        ]);
        assert_eq!(res, expected);
        cleanup();
    }
}

mod implicit_tag_tests {
    use crate::repository::open_connection;
    use crate::tags::repository::{get_all_tags_for_file, remove_implicit_tag_from_file};
    use crate::test::*;

    #[test]
    fn delete_implicit_tag_from_file_works() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1));
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        // Add implicit tag
        crate::test::imply_tag_on_file(tag_id, 1, 1);
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 1);
        // Delete the implicit tag
        remove_implicit_tag_from_file(tag_id, 1, &con).unwrap();
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 0);
        con.close().unwrap();
        cleanup();
    }
}

mod add_implicit_tag_to_folders_tests {
    use crate::repository::open_connection;
    use crate::tags::repository::{add_implicit_tag_to_folders, get_all_tags_for_folder};
    use crate::test::*;

    #[test]
    fn adds_tags_to_multiple_folders() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child1", Some(1)); // id 2
        create_folder_db_entry("child2", Some(1)); // id 3
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_folders(tag_id, &[2, 3], 1, &con).unwrap();
        let tags2 = get_all_tags_for_folder(2, &con).unwrap();
        let tags3 = get_all_tags_for_folder(3, &con).unwrap();
        assert_eq!(tags2.len(), 1);
        assert_eq!(tags2[0].tag_id, tag_id);
        assert_eq!(tags2[0].implicit_from_id, Some(1));
        assert_eq!(tags3.len(), 1);
        assert_eq!(tags3[0].tag_id, tag_id);
        assert_eq!(tags3[0].implicit_from_id, Some(1));
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn works_with_empty_slice() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_folders(tag_id, &[], 1, &con).unwrap();
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn works_with_single_folder() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_folders(tag_id, &[2], 1, &con).unwrap();
        let tags = get_all_tags_for_folder(2, &con).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_id, tag_id);
        assert_eq!(tags[0].implicit_from_id, Some(1));
        con.close().unwrap();
        cleanup();
    }
}

mod add_implicit_tag_to_files_tests {
    use crate::repository::open_connection;
    use crate::tags::repository::{add_implicit_tag_to_files, get_all_tags_for_file};
    use crate::test::*;

    #[test]
    fn adds_tags_to_multiple_files() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file1.txt", Some(1)); // id 1
        create_file_db_entry("file2.txt", Some(1)); // id 2
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_files(tag_id, &[1, 2], 1, &con).unwrap();
        let tags1 = get_all_tags_for_file(1, &con).unwrap();
        let tags2 = get_all_tags_for_file(2, &con).unwrap();
        assert_eq!(tags1.len(), 1);
        assert_eq!(tags1[0].tag_id, tag_id);
        assert_eq!(tags1[0].implicit_from_id, Some(1));
        assert_eq!(tags2.len(), 1);
        assert_eq!(tags2[0].tag_id, tag_id);
        assert_eq!(tags2[0].implicit_from_id, Some(1));
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn works_with_empty_slice() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_files(tag_id, &[], 1, &con).unwrap();
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn works_with_single_file() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_files(tag_id, &[1], 1, &con).unwrap();
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_id, tag_id);
        assert_eq!(tags[0].implicit_from_id, Some(1));
        con.close().unwrap();
        cleanup();
    }
}

mod add_implicit_tags_to_files_tests {
    use crate::repository::open_connection;
    use crate::tags::repository::{add_implicit_tags_to_files, get_all_tags_for_file};
    use crate::test::*;

    #[test]
    fn adds_multiple_tags_to_multiple_files() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file1.txt", Some(1)); // id 1
        create_file_db_entry("file2.txt", Some(1)); // id 2
        let tag_id1 = create_tag_db_entry("test_tag1");
        let tag_id2 = create_tag_db_entry("test_tag2");
        let con = open_connection();
        add_implicit_tags_to_files(&[1, 2], &[tag_id1, tag_id2], 1, &con).unwrap();
        let tags1 = get_all_tags_for_file(1, &con).unwrap();
        let tags2 = get_all_tags_for_file(2, &con).unwrap();
        assert_eq!(tags1.len(), 2);
        assert_eq!(tags2.len(), 2);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn does_nothing_when_file_ids_is_empty() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        // Should not add anything when file_ids is empty
        let result = add_implicit_tags_to_files(&[], &[tag_id], 1, &con);
        assert!(result.is_ok());
        // Verify no tags were added to the file
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 0, "No tags should have been added");
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn does_nothing_when_tag_ids_is_empty() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1
        let con = open_connection();
        // Should not add anything when tag_ids is empty
        let result = add_implicit_tags_to_files(&[1], &[], 1, &con);
        assert!(result.is_ok());
        // Verify no tags were added
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 0, "No tags should have been added");
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn does_not_add_unspecified_tags() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file1.txt", Some(1)); // id 1
        create_file_db_entry("file2.txt", Some(1)); // id 2
        let tag_id1 = create_tag_db_entry("test_tag1");
        let _tag_id2 = create_tag_db_entry("test_tag2");
        let con = open_connection();
        // Only add tag1 to file1
        add_implicit_tags_to_files(&[1], &[tag_id1], 1, &con).unwrap();
        let tags1 = get_all_tags_for_file(1, &con).unwrap();
        let tags2 = get_all_tags_for_file(2, &con).unwrap();
        assert_eq!(tags1.len(), 1);
        assert_eq!(tags1[0].tag_id, tag_id1);
        assert_eq!(tags2.len(), 0); // file2 should have no tags
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn handles_many_file_ids() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        // Create 1001 files to cross the 999 chunk boundary
        for i in 1..=1001 {
            create_file_db_entry(&format!("file{i}.txt"), Some(1));
        }
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        let file_ids: Vec<u32> = (1..=1001).collect();
        add_implicit_tags_to_files(&file_ids, &[tag_id], 1, &con).unwrap();
        // Verify first and last files have the tag (ensuring both chunks processed)
        let tags_first = get_all_tags_for_file(1, &con).unwrap();
        let tags_last = get_all_tags_for_file(1001, &con).unwrap();
        assert_eq!(tags_first.len(), 1);
        assert_eq!(tags_last.len(), 1);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn handles_many_tag_ids() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1
        // Create 1001 tags to cross the 999 chunk boundary
        let mut tag_ids = Vec::new();
        for i in 1..=1001 {
            tag_ids.push(create_tag_db_entry(&format!("tag{i}")));
        }
        let con = open_connection();
        add_implicit_tags_to_files(&[1], &tag_ids, 1, &con).unwrap();
        // Verify file has all tags
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 1001);
        con.close().unwrap();
        cleanup();
    }
}

mod batch_remove_implicit_tags_tests {
    use crate::repository::open_connection;
    use crate::tags::repository::{
        batch_remove_implicit_tags, get_all_tags_for_file, get_all_tags_for_folder,
    };
    use crate::test::*;

    #[test]
    fn removes_all_specified_tags() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1
        let tag_id = create_tag_db_entry("test_tag");
        imply_tag_on_file(tag_id, 1, 1);
        let con = open_connection();
        batch_remove_implicit_tags(&[1], &[], &[1], &con).unwrap();
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 0);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn does_not_touch_unspecified_entries() {
        init_db_folder();
        create_folder_db_entry("parent1", None); // id 1
        create_folder_db_entry("parent2", None); // id 2
        create_file_db_entry("file1.txt", Some(1)); // id 1
        create_file_db_entry("file2.txt", Some(2)); // id 2
        let tag_id = create_tag_db_entry("test_tag");
        imply_tag_on_file(tag_id, 1, 1);
        imply_tag_on_file(tag_id, 2, 2);
        let con = open_connection();
        // Remove only from file 1
        batch_remove_implicit_tags(&[1], &[], &[1], &con).unwrap();
        let tags1 = get_all_tags_for_file(1, &con).unwrap();
        let tags2 = get_all_tags_for_file(2, &con).unwrap();
        assert_eq!(tags1.len(), 0);
        assert_eq!(tags2.len(), 1); // file 2 should still have its tag
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn handles_many_file_ids() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file1.txt", Some(1)); // id 1
        create_file_db_entry("file2.txt", Some(1)); // id 2
        let tag_id = create_tag_db_entry("test_tag");
        // Add implicit tags to file 1 and file 2
        imply_tag_on_file(tag_id, 1, 1);
        imply_tag_on_file(tag_id, 2, 1);
        let con = open_connection();
        // Generate a large range of file ids including 1 and 2 (but many won't exist)
        let file_ids: Vec<u32> = (1..=5000).collect();
        batch_remove_implicit_tags(&file_ids, &[], &[1], &con).unwrap();
        let tags1 = get_all_tags_for_file(1, &con).unwrap();
        let tags2 = get_all_tags_for_file(2, &con).unwrap();
        assert_eq!(tags1.len(), 0);
        assert_eq!(tags2.len(), 0);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn handles_many_folder_ids() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child1", Some(1)); // id 2
        create_folder_db_entry("child2", Some(1)); // id 3
        let tag_id = create_tag_db_entry("test_tag");
        // Add implicit tags to folder 2 and folder 3
        imply_tag_on_folder(tag_id, 2, 1);
        imply_tag_on_folder(tag_id, 3, 1);
        let con = open_connection();
        // Generate a large range of folder ids including 2 and 3 (but many won't exist)
        let folder_ids: Vec<u32> = (1..=5000).collect();
        batch_remove_implicit_tags(&[], &folder_ids, &[1], &con).unwrap();
        let tags2 = get_all_tags_for_folder(2, &con).unwrap();
        let tags3 = get_all_tags_for_folder(3, &con).unwrap();
        assert_eq!(tags2.len(), 0);
        assert_eq!(tags3.len(), 0);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn handles_many_implicit_from_ids() {
        init_db_folder();
        create_folder_db_entry("parent1", None); // id 1
        create_folder_db_entry("parent2", None); // id 2
        create_file_db_entry("file.txt", None); // id 1
        let tag_id1 = create_tag_db_entry("test_tag1");
        let tag_id2 = create_tag_db_entry("test_tag2");
        // Add implicit tags from folder 1 and folder 2 (different tags to avoid unique constraint)
        imply_tag_on_file(tag_id1, 1, 1);
        imply_tag_on_file(tag_id2, 1, 2);
        let con = open_connection();
        // Verify we have 2 tags now
        let tags_before = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags_before.len(), 2);
        // Generate a large range of implicit_from_ids including 1 and 2 (but many won't exist)
        let implicit_from_ids: Vec<u32> = (1..=5000).collect();
        batch_remove_implicit_tags(&[1], &[], &implicit_from_ids, &con).unwrap();
        let tags_after = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags_after.len(), 0);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn does_nothing_when_implicit_from_ids_is_empty() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1
        let tag_id = create_tag_db_entry("test_tag");
        imply_tag_on_file(tag_id, 1, 1);
        let con = open_connection();
        // Should not remove anything when implicit_from_ids is empty
        let result = batch_remove_implicit_tags(&[1], &[1], &[], &con);
        assert!(result.is_ok());
        // Verify the tag still exists
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 1, "Tag should not have been removed");
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn does_nothing_when_file_and_folder_ids_empty() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1
        let tag_id = create_tag_db_entry("test_tag");
        imply_tag_on_file(tag_id, 1, 1);
        let con = open_connection();
        // Should not remove anything when both file_ids and folder_ids are empty
        let result = batch_remove_implicit_tags(&[], &[], &[1], &con);
        assert!(result.is_ok());
        // Verify the tag still exists
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 1, "Tag should not have been removed");
        con.close().unwrap();
        cleanup();
    }
}
