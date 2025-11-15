mod create_tag_tests {
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
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
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
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
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
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
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
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
    use crate::model::repository::{FileRecord, Tag};
    use crate::repository::file_repository::create_file;
    use crate::repository::open_connection;
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
        add_tag_to_file(1, 1, &con).unwrap();
        add_tag_to_file(1, 2, &con).unwrap();
        let res = get_tags_on_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            vec![
                Tag {
                    id: 1,
                    title: "test".to_string()
                },
                Tag {
                    id: 2,
                    title: "test2".to_string()
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
        let res = get_tags_on_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), res);
        cleanup();
    }
}

mod remove_tag_from_file_tests {
    use crate::model::file_types::FileTypes;
    use crate::model::repository::{FileRecord, Tag};
    use crate::repository::file_repository::create_file;
    use crate::repository::open_connection;
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
        remove_tag_from_file(1, 1, &con).unwrap();
        let tags = get_tags_on_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), tags);
        cleanup();
    }
}

mod get_tag_on_folder_tests {
    use crate::model::repository::{Folder, Tag};
    use crate::repository::folder_repository::create_folder;
    use crate::repository::open_connection;
    use crate::tags::repository::{add_tag_to_folder, create_tag, get_tags_on_folder};
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
        add_tag_to_folder(1, 1, &con).unwrap();
        add_tag_to_folder(1, 2, &con).unwrap();
        let res = get_tags_on_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            vec![
                Tag {
                    id: 1,
                    title: "test".to_string()
                },
                Tag {
                    id: 2,
                    title: "test2".to_string()
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
        let res = get_tags_on_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), res);
        cleanup();
    }
}

mod remove_tag_from_folder_tests {
    use crate::model::repository::{Folder, Tag};
    use crate::repository::folder_repository::create_folder;
    use crate::repository::open_connection;
    use crate::tags::repository::{create_tag, get_tags_on_folder, remove_tag_from_folder};
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
        remove_tag_from_folder(1, 1, &con).unwrap();
        let tags = get_tags_on_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), tags);
        cleanup();
    }
}

mod get_tags_on_files_tests {
    use std::collections::HashMap;

    use crate::tags::repository::get_tags_on_files;
    use crate::{model::repository::Tag, repository::open_connection, test::*};

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
        let res = get_tags_on_files(vec![1, 2, 3], &con).unwrap();
        con.close().unwrap();
        #[rustfmt::skip]
        let expected = HashMap::from([
            (1, vec![Tag {id: 1, title: "tag1".to_string()}, Tag {id: 2, title: "tag2".to_string()}]),
            (2, vec![Tag {id: 3, title: "tag3".to_string()}])
        ]);
        assert_eq!(res, expected);
        cleanup();
    }
}
