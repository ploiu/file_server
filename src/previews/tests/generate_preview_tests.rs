use crate::{
    previews::preview_dir,
    service::file_service::file_dir,
    test::{cleanup, init_db_folder},
};

#[test]
fn generate_preview_successfully_creates_preview_for_file() {
    init_db_folder();
    std::fs::copy("test_assets/test.png", format!("{}/test.png", file_dir()))
        .expect("Failed to copy test file to the file directory");
    cleanup();
}

#[test]
fn generate_preview_ignores_missing_file_from_db() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[test]
fn generate_preview_no_ffmpeg() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[test]
fn generate_preview_message_not_file_id() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[test]
fn generate_preview_ignores_missing_file_from_disk() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[test]
fn generate_preview_generates_for_image() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[test]
fn generate_preview_generates_for_video() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[test]
fn generate_preview_does_not_generate_for_other_file_types() {
    init_db_folder();
    crate::fail!();
    cleanup();
}
