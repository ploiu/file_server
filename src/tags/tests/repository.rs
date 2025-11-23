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
    use crate::tags::repository::{
        add_implicit_tag_to_file, add_implicit_tag_to_folder, get_all_tags_for_file,
        get_all_tags_for_folder, remove_implicit_tag_from_file, remove_implicit_tag_from_folder,
        upsert_implicit_tag_to_file, upsert_implicit_tag_to_folder,
    };
    use crate::test::*;

    #[test]
    fn add_implicit_tag_to_folder_works() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_folder(tag_id, 2, 1, &con).unwrap();
        let tags = get_all_tags_for_folder(2, &con).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_id, tag_id);
        assert_eq!(tags[0].implicit_from_id, Some(1));
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn add_implicit_tag_to_file_works() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1));
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_file(tag_id, 1, 1, &con).unwrap();
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_id, tag_id);
        assert_eq!(tags[0].implicit_from_id, Some(1));
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn upsert_implicit_tag_to_folder_replaces_existing() {
        init_db_folder();
        create_folder_db_entry("grandparent", None); // id 1
        create_folder_db_entry("parent", Some(1)); // id 2
        create_folder_db_entry("child", Some(2)); // id 3
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        // Add implicit tag from folder 1
        add_implicit_tag_to_folder(tag_id, 3, 1, &con).unwrap();
        // Upsert to change it to folder 2
        upsert_implicit_tag_to_folder(tag_id, 3, 2, &con).unwrap();
        let tags = get_all_tags_for_folder(3, &con).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_id, tag_id);
        assert_eq!(tags[0].implicit_from_id, Some(2));
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn upsert_implicit_tag_to_file_replaces_existing() {
        init_db_folder();
        create_folder_db_entry("grandparent", None); // id 1
        create_folder_db_entry("parent", Some(1)); // id 2
        create_file_db_entry("file.txt", Some(2));
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        // Add implicit tag from folder 1
        add_implicit_tag_to_file(tag_id, 1, 1, &con).unwrap();
        // Upsert to change it to folder 2
        upsert_implicit_tag_to_file(tag_id, 1, 2, &con).unwrap();
        let tags = get_all_tags_for_file(1, &con).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_id, tag_id);
        assert_eq!(tags[0].implicit_from_id, Some(2));
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn delete_implicit_tag_from_folder_works() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        // Add implicit tag
        add_implicit_tag_to_folder(tag_id, 2, 1, &con).unwrap();
        let tags = get_all_tags_for_folder(2, &con).unwrap();
        assert_eq!(tags.len(), 1);
        // Delete the implicit tag
        remove_implicit_tag_from_folder(tag_id, 2, &con).unwrap();
        let tags = get_all_tags_for_folder(2, &con).unwrap();
        assert_eq!(tags.len(), 0);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn delete_implicit_tag_from_file_works() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1));
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        // Add implicit tag
        add_implicit_tag_to_file(tag_id, 1, 1, &con).unwrap();
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

mod remove_implicit_tags_tests {
    use crate::repository::open_connection;
    use crate::tags::repository::{
        add_implicit_tag_to_file, add_implicit_tag_to_folder, get_all_tags_for_file,
        get_all_tags_for_folder, remove_implicit_tag_from_files, remove_implicit_tags_from_folders,
    };
    use crate::test::*;

    #[test]
    fn remove_implicit_tags_from_folders_works() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("folder1", Some(1)); // id 2
        create_folder_db_entry("folder2", Some(1)); // id 3
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_folder(tag_id, 2, 1, &con).unwrap();
        add_implicit_tag_to_folder(tag_id, 3, 1, &con).unwrap();
        // Remove tags inherited from folder 1
        remove_implicit_tags_from_folders(tag_id, 1, &con).unwrap();
        let tags2 = get_all_tags_for_folder(2, &con).unwrap();
        let tags3 = get_all_tags_for_folder(3, &con).unwrap();
        assert_eq!(tags2.len(), 0);
        assert_eq!(tags3.len(), 0);
        con.close().unwrap();
        cleanup();
    }

    #[test]
    fn remove_implicit_tags_from_files_works() {
        init_db_folder();
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file1.txt", Some(1));
        create_file_db_entry("file2.txt", Some(1));
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        add_implicit_tag_to_file(tag_id, 1, 1, &con).unwrap();
        add_implicit_tag_to_file(tag_id, 2, 1, &con).unwrap();
        // Remove tags inherited from folder 1
        remove_implicit_tag_from_files(tag_id, 1, &con).unwrap();
        let tags1 = get_all_tags_for_file(1, &con).unwrap();
        let tags2 = get_all_tags_for_file(2, &con).unwrap();
        assert_eq!(tags1.len(), 0);
        assert_eq!(tags2.len(), 0);
        con.close().unwrap();
        cleanup();
    }
}
