mod get_tag_tests {
    use crate::model::error::tag_errors::GetTagError;
    use crate::tags::service::{create_tag, get_tag};
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

mod update_tag_tests {
    use crate::model::error::tag_errors::UpdateTagError;
    use crate::model::response::TagApi;
    use crate::tags::service::{create_tag, get_tag, update_tag};
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

mod delete_tag_tests {
    use crate::model::error::tag_errors::GetTagError;
    use crate::tags::service::{create_tag, delete_tag, get_tag};
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

mod update_file_tag_test {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::model::file_types::FileTypes;
    use crate::model::repository::FileRecord;
    use crate::model::response::TagApi;

    use crate::tags::service::{create_tag, get_tags_on_file, update_file_tags};
    use crate::test::{cleanup, init_db_folder, now};

    #[test]
    fn update_file_tags_works() {
        init_db_folder();
        create_tag("test".to_string()).unwrap();
        FileRecord {
            id: None,
            name: "test_file".to_string(),
            parent_id: None,
            size: 0,
            create_date: now(),
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
            create_date: now(),
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
            create_date: now(),
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
            create_date: now(),
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
            create_date: now(),
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

mod update_folder_tag_test {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::model::repository::Folder;
    use crate::model::response::TagApi;
    use crate::repository::{folder_repository, open_connection};
    use crate::tags::service::{create_tag, get_tags_on_folder, update_folder_tags};
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

mod get_tags_on_file_tests {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::tags::service::get_tags_on_file;
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn throws_error_if_file_not_found() {
        init_db_folder();
        let err = get_tags_on_file(1).unwrap_err();
        assert_eq!(TagRelationError::FileNotFound, err);
        cleanup();
    }
}

mod get_tags_on_folder_tests {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::tags::service::get_tags_on_folder;
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn throws_error_if_file_not_found() {
        init_db_folder();
        let err = get_tags_on_folder(1).unwrap_err();
        assert_eq!(TagRelationError::FileNotFound, err);
        cleanup();
    }
}
