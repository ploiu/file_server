use crate::repository::{file_repository, folder_repository, initialize_db, open_connection};
use crate::service::file_service::file_dir;
use crate::test::{cleanup, create_file_disk, create_folder_disk};

mod generate_database_from_files_basic {
    use super::*;

    #[test]
    fn empty_files_directory_returns_ok() {
        cleanup();
        // Create empty files directory
        std::fs::create_dir_all(file_dir()).unwrap();
        initialize_db().unwrap();

        let con = open_connection();
        // Should not have created any folders or files
        let folders = folder_repository::get_child_folders(None, &con).unwrap();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        assert!(folders.is_empty());
        assert!(files.is_empty());

        cleanup();
    }

    #[test]
    fn missing_files_directory_returns_ok() {
        cleanup();
        // Don't create files directory at all
        initialize_db().unwrap();

        let con = open_connection();
        // Should not have created any folders or files
        let folders = folder_repository::get_child_folders(None, &con).unwrap();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        assert!(folders.is_empty());
        assert!(files.is_empty());

        cleanup();
    }
}

mod generate_database_from_files_single_level {
    use super::*;

    #[test]
    fn creates_single_file_at_root() {
        cleanup();
        create_file_disk("test.txt", "test content");
        initialize_db().unwrap();

        let con = open_connection();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "test.txt");

        cleanup();
    }

    #[test]
    fn creates_single_folder_at_root() {
        cleanup();
        create_folder_disk("folder1");
        initialize_db().unwrap();

        let con = open_connection();
        let folders = folder_repository::get_child_folders(None, &con).unwrap();
        con.close().unwrap();

        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "folder1");

        cleanup();
    }

    #[test]
    fn creates_multiple_files_at_root() {
        cleanup();
        create_file_disk("test1.txt", "content1");
        create_file_disk("test2.png", "content2");
        create_file_disk("test3.mp4", "content3");
        initialize_db().unwrap();

        let con = open_connection();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        assert_eq!(files.len(), 3);
        let file_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert!(file_names.contains(&"test1.txt"));
        assert!(file_names.contains(&"test2.png"));
        assert!(file_names.contains(&"test3.mp4"));

        cleanup();
    }

    #[test]
    fn creates_multiple_folders_at_root() {
        cleanup();
        create_folder_disk("folder1");
        create_folder_disk("folder2");
        create_folder_disk("folder3");
        initialize_db().unwrap();

        let con = open_connection();
        let folders = folder_repository::get_child_folders(None, &con).unwrap();
        con.close().unwrap();

        assert_eq!(folders.len(), 3);
        let folder_names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert!(folder_names.contains(&"folder1"));
        assert!(folder_names.contains(&"folder2"));
        assert!(folder_names.contains(&"folder3"));

        cleanup();
    }

    #[test]
    fn creates_files_and_folders_at_root() {
        cleanup();
        create_folder_disk("folder1");
        create_folder_disk("folder2");
        create_file_disk("file1.txt", "content1");
        create_file_disk("file2.png", "content2");
        initialize_db().unwrap();

        let con = open_connection();
        let folders = folder_repository::get_child_folders(None, &con).unwrap();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        assert_eq!(folders.len(), 2);
        assert_eq!(files.len(), 2);

        cleanup();
    }
}

mod generate_database_from_files_nested {
    use super::*;

    #[test]
    fn creates_nested_folder_with_file() {
        cleanup();
        create_folder_disk("parent");
        create_file_disk("parent/child.txt", "child content");
        initialize_db().unwrap();

        let con = open_connection();
        let root_folders = folder_repository::get_child_folders(None, &con).unwrap();
        assert_eq!(root_folders.len(), 1);
        assert_eq!(root_folders[0].name, "parent");

        let parent_id = root_folders[0].id.unwrap();
        let child_files = folder_repository::get_child_files(&[parent_id], &con).unwrap();
        con.close().unwrap();

        assert_eq!(child_files.len(), 1);
        assert_eq!(child_files[0].name, "child.txt");
        assert_eq!(child_files[0].parent_id, Some(parent_id));

        cleanup();
    }

    #[test]
    fn creates_nested_folders() {
        cleanup();
        create_folder_disk("parent/child");
        create_file_disk("parent/child/grandchild.txt", "content");
        initialize_db().unwrap();

        let con = open_connection();
        let root_folders = folder_repository::get_child_folders(None, &con).unwrap();
        assert_eq!(root_folders.len(), 1);

        let parent_id = root_folders[0].id.unwrap();
        let child_folders = folder_repository::get_child_folders(Some(parent_id), &con).unwrap();
        assert_eq!(child_folders.len(), 1);
        // Note: folder name is the full path in the database
        assert_eq!(child_folders[0].name, "parent/child");

        let child_id = child_folders[0].id.unwrap();
        let grandchild_files = folder_repository::get_child_files(&[child_id], &con).unwrap();
        con.close().unwrap();

        assert_eq!(grandchild_files.len(), 1);
        assert_eq!(grandchild_files[0].name, "grandchild.txt");

        cleanup();
    }
}

mod generate_database_from_files_deep_nesting {
    use super::*;

    #[test]
    fn handles_6_levels_deep() {
        cleanup();
        // Create a 6-level deep structure
        create_folder_disk("level1/level2/level3/level4/level5/level6");
        create_file_disk(
            "level1/level2/level3/level4/level5/level6/deep_file.txt",
            "deep content",
        );
        initialize_db().unwrap();

        let con = open_connection();

        // Verify level 1
        let level1_folders = folder_repository::get_child_folders(None, &con).unwrap();
        assert_eq!(level1_folders.len(), 1);
        assert_eq!(level1_folders[0].name, "level1");
        let level1_id = level1_folders[0].id.unwrap();

        // Verify level 2 (folder name is full path from root)
        let level2_folders = folder_repository::get_child_folders(Some(level1_id), &con).unwrap();
        assert_eq!(level2_folders.len(), 1);
        assert_eq!(level2_folders[0].name, "level1/level2");
        let level2_id = level2_folders[0].id.unwrap();

        // Verify level 3
        let level3_folders = folder_repository::get_child_folders(Some(level2_id), &con).unwrap();
        assert_eq!(level3_folders.len(), 1);
        assert_eq!(level3_folders[0].name, "level1/level2/level3");
        let level3_id = level3_folders[0].id.unwrap();

        // Verify level 4
        let level4_folders = folder_repository::get_child_folders(Some(level3_id), &con).unwrap();
        assert_eq!(level4_folders.len(), 1);
        assert_eq!(level4_folders[0].name, "level1/level2/level3/level4");
        let level4_id = level4_folders[0].id.unwrap();

        // Verify level 5
        let level5_folders = folder_repository::get_child_folders(Some(level4_id), &con).unwrap();
        assert_eq!(level5_folders.len(), 1);
        assert_eq!(level5_folders[0].name, "level1/level2/level3/level4/level5");
        let level5_id = level5_folders[0].id.unwrap();

        // Verify level 6
        let level6_folders = folder_repository::get_child_folders(Some(level5_id), &con).unwrap();
        assert_eq!(level6_folders.len(), 1);
        assert_eq!(
            level6_folders[0].name,
            "level1/level2/level3/level4/level5/level6"
        );
        let level6_id = level6_folders[0].id.unwrap();

        // Verify deepest file
        let deep_files = folder_repository::get_child_files(&[level6_id], &con).unwrap();
        con.close().unwrap();

        assert_eq!(deep_files.len(), 1);
        assert_eq!(deep_files[0].name, "deep_file.txt");
        assert_eq!(deep_files[0].parent_id, Some(level6_id));

        cleanup();
    }
}

mod generate_database_from_files_file_properties {
    use crate::model::file_types::FileTypes;

    use super::*;

    #[test]
    fn correctly_determines_file_type() {
        cleanup();
        create_file_disk("test.txt", "text");
        create_file_disk("test.png", "image");
        create_file_disk("test.mp4", "video");
        initialize_db().unwrap();

        let con = open_connection();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        let txt_file = files.iter().find(|f| f.name == "test.txt").unwrap();
        let png_file = files.iter().find(|f| f.name == "test.png").unwrap();
        let mp4_file = files.iter().find(|f| f.name == "test.mp4").unwrap();

        assert_eq!(txt_file.file_type, FileTypes::Text);
        assert_eq!(png_file.file_type, FileTypes::Image);
        assert_eq!(mp4_file.file_type, FileTypes::Video);

        cleanup();
    }

    #[test]
    fn correctly_stores_file_size() {
        cleanup();
        let content = "test content with specific size";
        create_file_disk("sized.txt", content);
        initialize_db().unwrap();

        let con = open_connection();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size, content.len() as u64);

        cleanup();
    }
}

mod generate_database_from_files_complex_structures {
    use super::*;

    #[test]
    fn creates_breadth_structure() {
        cleanup();
        // Create multiple folders with multiple files each
        create_folder_disk("folder_a");
        create_folder_disk("folder_b");
        create_folder_disk("folder_c");
        create_file_disk("folder_a/file_a1.txt", "content");
        create_file_disk("folder_a/file_a2.txt", "content");
        create_file_disk("folder_b/file_b1.txt", "content");
        create_file_disk("folder_c/file_c1.txt", "content");
        create_file_disk("folder_c/file_c2.txt", "content");
        create_file_disk("folder_c/file_c3.txt", "content");
        initialize_db().unwrap();

        let con = open_connection();
        let root_folders = folder_repository::get_child_folders(None, &con).unwrap();
        assert_eq!(root_folders.len(), 3);

        // Find each folder by name and verify its contents
        let folder_a = root_folders.iter().find(|f| f.name == "folder_a").unwrap();
        let folder_b = root_folders.iter().find(|f| f.name == "folder_b").unwrap();
        let folder_c = root_folders.iter().find(|f| f.name == "folder_c").unwrap();

        let files_a = folder_repository::get_child_files(&[folder_a.id.unwrap()], &con).unwrap();
        let files_b = folder_repository::get_child_files(&[folder_b.id.unwrap()], &con).unwrap();
        let files_c = folder_repository::get_child_files(&[folder_c.id.unwrap()], &con).unwrap();
        con.close().unwrap();

        assert_eq!(files_a.len(), 2);
        assert_eq!(files_b.len(), 1);
        assert_eq!(files_c.len(), 3);

        cleanup();
    }
}

mod generate_database_existing_db {
    use super::*;
    use crate::test::init_db_folder;

    #[test]
    fn does_not_regenerate_when_db_exists() {
        cleanup();
        // First, create database with init_db_folder (which uses initialize_db)
        init_db_folder();

        // Manually create a file in the files directory AFTER db is initialized
        create_file_disk("new_file.txt", "new content");

        // Call initialize_db again - it should NOT regenerate from files
        initialize_db().unwrap();

        let con = open_connection();
        // Verify the new_file.txt is NOT in the database
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        // The file should not be in the database because we didn't regenerate
        assert!(
            !files.iter().any(|f| f.name == "new_file.txt"),
            "File should not be in database because db already existed"
        );

        cleanup();
    }

    #[test]
    fn preserves_existing_data() {
        cleanup();
        // First create db and add some data
        init_db_folder();

        // Create a file entry in the db (not on disk)
        let con = open_connection();
        file_repository::create_file(
            &crate::model::repository::FileRecord {
                id: None,
                name: "existing_file.txt".to_string(),
                parent_id: None,
                create_date: chrono::offset::Local::now().naive_local(),
                size: 100,
                file_type: crate::model::file_types::FileTypes::Text,
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();

        // Create a file on disk that we want to make sure doesn't get added
        create_file_disk("disk_only.txt", "disk content");

        // Call initialize_db again
        initialize_db().unwrap();

        let con = open_connection();
        let files = folder_repository::get_child_files(&[], &con).unwrap();
        con.close().unwrap();

        // The existing file should still be there
        assert!(
            files.iter().any(|f| f.name == "existing_file.txt"),
            "Existing file should still be in database"
        );
        // The disk-only file should NOT be added
        assert!(
            !files.iter().any(|f| f.name == "disk_only.txt"),
            "Disk-only file should not have been added to existing database"
        );

        cleanup();
    }
}

mod generate_database_verifies_all_files {
    use super::*;

    #[test]
    fn all_files_at_various_levels_are_in_database() {
        cleanup();
        // Create a mixed structure
        create_file_disk("root1.txt", "root1");
        create_file_disk("root2.png", "root2");
        create_folder_disk("folder1");
        create_file_disk("folder1/level1_file1.txt", "l1f1");
        create_file_disk("folder1/level1_file2.txt", "l1f2");
        create_folder_disk("folder1/subfolder");
        create_file_disk("folder1/subfolder/level2_file.txt", "l2f");
        create_folder_disk("folder2");
        create_file_disk("folder2/another.txt", "another");

        initialize_db().unwrap();

        let con = open_connection();

        // Check root files
        let root_files = folder_repository::get_child_files(&[], &con).unwrap();
        assert_eq!(root_files.len(), 2);
        assert!(root_files.iter().any(|f| f.name == "root1.txt"));
        assert!(root_files.iter().any(|f| f.name == "root2.png"));

        // Check root folders
        let root_folders = folder_repository::get_child_folders(None, &con).unwrap();
        assert_eq!(root_folders.len(), 2);

        // Check folder1 contents
        let folder1 = root_folders.iter().find(|f| f.name == "folder1").unwrap();
        let folder1_id = folder1.id.unwrap();
        let folder1_files = folder_repository::get_child_files(&[folder1_id], &con).unwrap();
        assert_eq!(folder1_files.len(), 2);
        assert!(folder1_files.iter().any(|f| f.name == "level1_file1.txt"));
        assert!(folder1_files.iter().any(|f| f.name == "level1_file2.txt"));

        // Check subfolder contents
        let subfolder = folder_repository::get_child_folders(Some(folder1_id), &con).unwrap();
        assert_eq!(subfolder.len(), 1);
        let subfolder_id = subfolder[0].id.unwrap();
        let subfolder_files = folder_repository::get_child_files(&[subfolder_id], &con).unwrap();
        assert_eq!(subfolder_files.len(), 1);
        assert_eq!(subfolder_files[0].name, "level2_file.txt");

        // Check folder2 contents
        let folder2 = root_folders.iter().find(|f| f.name == "folder2").unwrap();
        let folder2_id = folder2.id.unwrap();
        let folder2_files = folder_repository::get_child_files(&[folder2_id], &con).unwrap();
        con.close().unwrap();

        assert_eq!(folder2_files.len(), 1);
        assert_eq!(folder2_files[0].name, "another.txt");

        cleanup();
    }
}
