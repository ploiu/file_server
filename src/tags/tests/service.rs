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
    use crate::model::response::TaggedItemApi;

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
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: None,
                    title: "new tag".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();
        let expected = vec![
            TaggedItemApi {
                tag_id: Some(1),
                title: "test".to_string(),
                implicit_from: None,
            },
            TaggedItemApi {
                tag_id: Some(2),
                title: "new tag".to_string(),
                implicit_from: None,
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
            vec![TaggedItemApi {
                tag_id: None,
                title: "test".to_string(),
                implicit_from: None,
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
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_file(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].tag_id, Some(1));
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
                TaggedItemApi {
                    tag_id: None,
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: None,
                    title: "test".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_file(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].tag_id, Some(1));
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
            vec![TaggedItemApi {
                tag_id: None,
                title: "test".to_string(),
                implicit_from: None,
            }],
        )
        .unwrap();

        // Now update with both the id and a new tag with same name
        update_file_tags(
            1,
            vec![
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: None,
                    title: "test".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_file(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].tag_id, Some(1));
        assert_eq!(actual[0].title, "test");
        cleanup();
    }
}

mod update_folder_tag_test {
    use crate::model::error::tag_errors::TagRelationError;
    use crate::model::repository::Folder;
    use crate::model::response::TaggedItemApi;
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
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: None,
                    title: "new tag".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();
        let expected = vec![
            TaggedItemApi {
                tag_id: Some(1),
                title: "test".to_string(),
                implicit_from: None,
            },
            TaggedItemApi {
                tag_id: Some(2),
                title: "new tag".to_string(),
                implicit_from: None,
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
            vec![TaggedItemApi {
                tag_id: None,
                title: "test".to_string(),
                implicit_from: None,
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
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_folder(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].tag_id, Some(1));
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
                TaggedItemApi {
                    tag_id: None,
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: None,
                    title: "test".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_folder(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].tag_id, Some(1));
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
            vec![TaggedItemApi {
                tag_id: None,
                title: "test".to_string(),
                implicit_from: None,
            }],
        )
        .unwrap();

        // Now update with both the id and a new tag with same name
        update_folder_tags(
            1,
            vec![
                TaggedItemApi {
                    tag_id: Some(1),
                    title: "test".to_string(),
                    implicit_from: None,
                },
                TaggedItemApi {
                    tag_id: None,
                    title: "test".to_string(),
                    implicit_from: None,
                },
            ],
        )
        .unwrap();

        let actual = get_tags_on_folder(1).unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].tag_id, Some(1));
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

mod pass_tags_to_children_tests {
    use crate::model::response::TaggedItemApi;
    use crate::tags::service::{get_tags_on_file, get_tags_on_folder, pass_tags_to_children};
    use crate::test::{
        cleanup, create_file_db_entry, create_folder_db_entry, create_tag_folder, init_db_folder,
    };

    #[test]
    fn should_imply_tag_to_descendant_folders() {
        init_db_folder();
        // Create folder hierarchy: parent -> child -> grandchild
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2
        create_folder_db_entry("grandchild", Some(2)); // id 3

        // Add tag to parent
        create_tag_folder("test_tag", 1);

        // Pass tags to children
        pass_tags_to_children(1).unwrap();

        // Check child has implicit tag
        let child_tags = get_tags_on_folder(2).unwrap();
        assert_eq!(child_tags.len(), 1);
        assert_eq!(child_tags[0].tag_id, Some(1));
        assert_eq!(child_tags[0].title, "test_tag");
        assert_eq!(child_tags[0].implicit_from, Some(1));

        // Check grandchild has implicit tag
        let grandchild_tags = get_tags_on_folder(3).unwrap();
        assert_eq!(grandchild_tags.len(), 1);
        assert_eq!(grandchild_tags[0].tag_id, Some(1));
        assert_eq!(grandchild_tags[0].title, "test_tag");
        assert_eq!(grandchild_tags[0].implicit_from, Some(1));

        cleanup();
    }

    #[test]
    fn should_imply_tag_to_descendant_files() {
        init_db_folder();
        // Create folder hierarchy: parent -> child
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2

        // Create files in folders
        create_file_db_entry("file1.txt", Some(1)); // id 1
        create_file_db_entry("file2.txt", Some(2)); // id 2

        // Add tag to parent
        create_tag_folder("test_tag", 1);

        // Pass tags to children
        pass_tags_to_children(1).unwrap();

        // Check file in parent has implicit tag
        let file1_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file1_tags.len(), 1);
        assert_eq!(file1_tags[0].tag_id, Some(1));
        assert_eq!(file1_tags[0].title, "test_tag");
        assert_eq!(file1_tags[0].implicit_from, Some(1));

        // Check file in child has implicit tag
        let file2_tags = get_tags_on_file(2).unwrap();
        assert_eq!(file2_tags.len(), 1);
        assert_eq!(file2_tags[0].tag_id, Some(1));
        assert_eq!(file2_tags[0].title, "test_tag");
        assert_eq!(file2_tags[0].implicit_from, Some(1));

        cleanup();
    }

    #[test]
    fn should_not_override_explicit_tags_on_folders() {
        init_db_folder();
        // Create folder hierarchy: parent -> child
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2

        // Create tag and add explicitly to both
        use crate::test::create_tag_db_entry;
        use crate::repository::open_connection;
        use crate::tags::repository as tag_repository;
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(1, tag_id, &con).unwrap();
        tag_repository::add_explicit_tag_to_folder(2, tag_id, &con).unwrap();
        con.close().unwrap();

        // Pass tags to children
        pass_tags_to_children(1).unwrap();

        // Check child still has explicit tag (not implicit)
        let child_tags = get_tags_on_folder(2).unwrap();
        assert_eq!(child_tags.len(), 1);
        assert_eq!(child_tags[0].tag_id, Some(tag_id));
        assert_eq!(child_tags[0].title, "test_tag");
        assert_eq!(child_tags[0].implicit_from, None); // Still explicit

        cleanup();
    }

    #[test]
    fn should_not_override_explicit_tags_on_files() {
        init_db_folder();
        // Create folder with file
        create_folder_db_entry("parent", None); // id 1
        create_file_db_entry("file.txt", Some(1)); // id 1

        // Create tag and add explicitly to both
        use crate::test::create_tag_db_entry;
        use crate::repository::open_connection;
        use crate::tags::repository as tag_repository;
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(1, tag_id, &con).unwrap();
        tag_repository::add_explicit_tag_to_file(1, tag_id, &con).unwrap();
        con.close().unwrap();

        // Pass tags to children
        pass_tags_to_children(1).unwrap();

        // Check file still has explicit tag (not implicit)
        let file_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file_tags.len(), 1);
        assert_eq!(file_tags[0].tag_id, Some(tag_id));
        assert_eq!(file_tags[0].title, "test_tag");
        assert_eq!(file_tags[0].implicit_from, None); // Still explicit

        cleanup();
    }

    #[test]
    fn should_remove_implicit_tags_when_folder_tag_removed() {
        init_db_folder();
        // Create folder hierarchy: parent -> child
        create_folder_db_entry("parent", None); // id 1
        create_folder_db_entry("child", Some(1)); // id 2

        // Add tag to parent and propagate
        create_tag_folder("test_tag", 1);
        pass_tags_to_children(1).unwrap();

        // Verify child has implicit tag
        let child_tags = get_tags_on_folder(2).unwrap();
        assert_eq!(child_tags.len(), 1);
        assert_eq!(child_tags[0].implicit_from, Some(1));

        // Remove the tag explicitly from parent
        use crate::repository::open_connection;
        use crate::tags::repository as tag_repository;
        let con = open_connection();
        tag_repository::remove_explicit_tag_from_folder(1, 1, &con).unwrap();
        con.close().unwrap();

        // Propagate the change
        pass_tags_to_children(1).unwrap();

        // Check child no longer has the tag
        let child_tags = get_tags_on_folder(2).unwrap();
        assert_eq!(child_tags.len(), 0);

        cleanup();
    }

    #[test]
    fn should_reinherit_from_higher_ancestor_when_tag_removed() {
        init_db_folder();
        // Create folder hierarchy: grandparent -> parent -> child
        create_folder_db_entry("grandparent", None); // id 1
        create_folder_db_entry("parent", Some(1)); // id 2
        create_folder_db_entry("child", Some(2)); // id 3

        // Create tag and add explicitly to both grandparent and parent
        use crate::test::create_tag_db_entry;
        use crate::repository::open_connection;
        use crate::tags::repository as tag_repository;
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(1, tag_id, &con).unwrap();
        tag_repository::add_explicit_tag_to_folder(2, tag_id, &con).unwrap();
        con.close().unwrap();

        pass_tags_to_children(1).unwrap();
        pass_tags_to_children(2).unwrap();

        // Child should inherit from parent (closer ancestor)
        let child_tags = get_tags_on_folder(3).unwrap();
        assert_eq!(child_tags.len(), 1);
        assert_eq!(child_tags[0].implicit_from, Some(2));

        // Remove tag from parent
        use crate::tags::service::update_folder_tags;
        update_folder_tags(2, vec![]).unwrap();

        // Child should now inherit from grandparent
        let child_tags = get_tags_on_folder(3).unwrap();
        assert_eq!(child_tags.len(), 1);
        assert_eq!(child_tags[0].tag_id, Some(tag_id));
        assert_eq!(child_tags[0].implicit_from, Some(1));

        cleanup();
    }

    #[test]
    fn should_inherit_from_closest_ancestor_folder() {
        init_db_folder();
        // Create folder hierarchy: top -> middle -> bottom
        create_folder_db_entry("top", None); // id 1
        create_folder_db_entry("middle", Some(1)); // id 2
        create_folder_db_entry("bottom", Some(2)); // id 3

        // Create tag once
        use crate::test::create_tag_db_entry;
        use crate::repository::open_connection;
        use crate::tags::repository as tag_repository;
        let tag_id = create_tag_db_entry("test_tag");
        
        // Add tag to bottom first
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(3, tag_id, &con).unwrap();
        con.close().unwrap();
        pass_tags_to_children(3).unwrap();

        // Then add same tag to middle
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(2, tag_id, &con).unwrap();
        con.close().unwrap();
        pass_tags_to_children(2).unwrap();

        // Bottom should still have it as explicit
        let bottom_tags = get_tags_on_folder(3).unwrap();
        assert_eq!(bottom_tags.len(), 1);
        assert_eq!(bottom_tags[0].implicit_from, None); // Explicit

        cleanup();
    }

    #[test]
    fn should_inherit_from_closest_ancestor_file() {
        init_db_folder();
        // Create folder hierarchy: top -> middle -> bottom (with file)
        create_folder_db_entry("top", None); // id 1
        create_folder_db_entry("middle", Some(1)); // id 2
        create_folder_db_entry("bottom", Some(2)); // id 3
        create_file_db_entry("file.png", Some(3));

        // Create tag once
        use crate::test::create_tag_db_entry;
        use crate::repository::open_connection;
        use crate::tags::repository as tag_repository;
        let tag_id = create_tag_db_entry("test_tag");

        // Add tag to bottom
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(3, tag_id, &con).unwrap();
        con.close().unwrap();
        pass_tags_to_children(3).unwrap();

        // File should inherit from bottom
        let file_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file_tags.len(), 1);
        assert_eq!(file_tags[0].implicit_from, Some(3));

        // Add same tag to middle - file should still inherit from bottom (closer)
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(2, tag_id, &con).unwrap();
        con.close().unwrap();
        pass_tags_to_children(2).unwrap();

        // File should still inherit from bottom (id 3), not middle (id 2)
        let file_tags = get_tags_on_file(1).unwrap();
        assert_eq!(file_tags.len(), 1);
        assert_eq!(file_tags[0].implicit_from, Some(3));

        cleanup();
    }

    #[test]
    fn removing_tag_from_distant_ancestor_should_not_affect_closer_inheritance() {
        init_db_folder();
        // Create folder hierarchy: top -> middle -> bottom
        create_folder_db_entry("top", None); // id 1
        create_folder_db_entry("middle", Some(1)); // id 2
        create_folder_db_entry("bottom", Some(2)); // id 3

        // Add tag to all three levels
        use crate::test::create_tag_db_entry;
        use crate::repository::open_connection;
        use crate::tags::repository as tag_repository;
        let tag_id = create_tag_db_entry("test_tag");
        let con = open_connection();
        tag_repository::add_explicit_tag_to_folder(1, tag_id, &con).unwrap();
        tag_repository::add_explicit_tag_to_folder(2, tag_id, &con).unwrap();
        tag_repository::add_explicit_tag_to_folder(3, tag_id, &con).unwrap();
        con.close().unwrap();

        pass_tags_to_children(1).unwrap();
        pass_tags_to_children(2).unwrap();
        pass_tags_to_children(3).unwrap();

        // Bottom should have explicit tag
        let bottom_tags = get_tags_on_folder(3).unwrap();
        assert_eq!(bottom_tags.len(), 1);
        assert_eq!(bottom_tags[0].implicit_from, None);

        // Remove tag from top - bottom should still have it explicitly
        use crate::tags::service::update_folder_tags;
        update_folder_tags(1, vec![]).unwrap();

        let bottom_tags = get_tags_on_folder(3).unwrap();
        assert_eq!(bottom_tags.len(), 1);
        assert_eq!(bottom_tags[0].implicit_from, None);

        // Remove tag from middle - bottom should still have it explicitly
        update_folder_tags(2, vec![]).unwrap();

        let bottom_tags = get_tags_on_folder(3).unwrap();
        assert_eq!(bottom_tags.len(), 1);
        assert_eq!(bottom_tags[0].implicit_from, None);

        cleanup();
    }
}
