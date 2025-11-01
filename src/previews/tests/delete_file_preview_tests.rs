use crate::previews::{delete_file_preview, preview_dir};
use crate::test::{cleanup, create_file_preview};
use std::path::Path;

#[test]
fn should_remove_the_preview_from_the_disk() {
    create_file_preview(1);
    let preview_path = format!("{}/1.png", preview_dir());
    let preview_path = Path::new(&preview_path);
    assert!(preview_path.exists());
    delete_file_preview(1);
    assert!(!preview_path.exists());
    cleanup();
}

#[test]
fn should_not_panic_if_no_preview() {
    let nonexistent_path = format!("{}/9999.png", preview_dir());
    let nonexistent_path = Path::new(&nonexistent_path);
    assert!(!nonexistent_path.exists());
    delete_file_preview(9999); // should not exist
    cleanup();
}
