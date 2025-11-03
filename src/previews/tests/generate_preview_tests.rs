use std::path::Path;

use rocket::tokio;

use crate::{
    previews::{self, preview_dir},
    service::file_service::file_dir,
    test::{self, cleanup, create_file_db_entry, init_db_folder},
};

#[tokio::test]
async fn generate_preview_successfully_creates_preview_for_image() {
    init_db_folder();
    std::fs::copy("test_assets/test.png", format!("{}/test.png", file_dir()))
        .expect("Failed to copy test file to the file directory");
    create_file_db_entry("test.png", None);
    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    assert!(
        !preview_path.exists(),
        "Preview should not exist before it's generated"
    );
    let res = previews::generate_preview("1".to_string()).await;
    assert!(res);
    assert!(
        preview_path.exists(),
        "Preview should exist after it's generated!"
    );
    cleanup();
}

#[tokio::test]
async fn generate_preview_ignores_missing_file_from_db() {
    init_db_folder();
    // Place a file on disk but don't create a DB entry for it.
    std::fs::copy("test_assets/test.png", format!("{}/test.png", file_dir()))
        .expect("Failed to copy test file to the file directory");
    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    let res = previews::generate_preview("1".to_string()).await;
    assert!(
        res,
        "generate_preview should return true when the file is missing from the DB so that it doesn't get re-queued"
    );
    assert!(
        !preview_path.exists(),
        "Preview should not be created when the DB entry is missing"
    );
    cleanup();
}

#[tokio::test]
async fn generate_preview_when_message_is_not_file_id() {
    init_db_folder();
    // Ensure no preview exists before running
    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    let res = previews::generate_preview("not-a-file-id".to_string()).await;
    assert!(
        res,
        "generate_preview should return true for messages that are not file ids"
    );
    assert!(
        !preview_path.exists(),
        "No preview should be created for an invalid message"
    );

    cleanup();
}

#[tokio::test]
async fn generate_preview_ignores_missing_file_from_disk() {
    init_db_folder();
    // Create a DB entry but do NOT place the file on disk.
    create_file_db_entry("test.png", None);
    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    let res = previews::generate_preview("1".to_string()).await;
    assert!(
        res,
        "generate_preview should return true when the file is missing from disk so that it doesn't get re-queued"
    );
    assert!(
        !preview_path.exists(),
        "Preview should not be created when the source file is missing from disk"
    );
    cleanup();
}

#[tokio::test]
async fn generate_preview_generates_for_video() {
    init_db_folder();
    std::fs::copy("test_assets/test.mp4", format!("{}/test.mp4", file_dir()))
        .expect("Failed to copy test file to the file directory");
    create_file_db_entry("test.mp4", None);

    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    assert!(
        !preview_path.exists(),
        "Preview should not exist before it's generated"
    );

    let res = previews::generate_preview("1".to_string()).await;
    assert!(res);
    assert!(
        preview_path.exists(),
        "Preview should exist after it's generated!"
    );

    cleanup();
}

#[tokio::test]
async fn generate_preview_generates_gif_as_video() {
    init_db_folder();
    std::fs::copy("test_assets/test.gif", format!("{}/test.gif", file_dir()))
        .expect("Failed to copy test file to the file directory");
    create_file_db_entry("test.gif", None);

    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    assert!(
        !preview_path.exists(),
        "Preview should not exist before it's generated"
    );

    let res = previews::generate_preview("1".to_string()).await;
    assert!(res);
    assert!(
        preview_path.exists(),
        "Preview should exist after it's generated!"
    );

    cleanup();
}

#[tokio::test]
async fn generate_preview_does_not_generate_for_other_file_types() {
    init_db_folder();
    // Create a non-image/video file on disk and a corresponding DB record.
    test::create_file_disk("test.txt", "asdfasdf");
    test::create_file_db_entry("test.txt", None);

    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    let res = previews::generate_preview("1".to_string()).await;
    assert!(
        res,
        "generate_preview should return true for unsupported file types so it doesn't get re-queued"
    );
    assert!(
        !preview_path.exists(),
        "Preview should not be created for unsupported file types"
    );

    cleanup();
}

#[tokio::test]
async fn generate_preview_does_not_overwrite_existing_preview() {
    init_db_folder();
    test::create_file_disk("test.png", "fake image contents");
    create_file_db_entry("test.png", None);
    test::create_file_preview(1);
    
    let res = previews::generate_preview("1".to_string()).await;
    
    assert!(res, "generate_preview should return true when preview already exists");
    
    let preview_path = format!("{}/1.png", preview_dir());
    let preview_contents = std::fs::read(&preview_path)
        .expect("Failed to read preview file");
    assert_eq!(
        preview_contents,
        vec![0x01, 0x02, 0x03],
        "Preview should not be overwritten when it already exists"
    );
    
    cleanup();
}
