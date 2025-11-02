use rocket::tokio;

use crate::model::error::file_errors::GetPreviewError;
use crate::previews::get_file_preview;
use crate::test::create_file_preview;
use crate::test::{cleanup, init_db_folder};

#[tokio::test]
#[cfg(not(ci))]
async fn should_return_preview_if_exists() {
    init_db_folder();
    create_file_preview(1);
    let expected: Vec<u8> = vec![0x01, 0x02, 0x03];
    let actual = get_file_preview(1).await.unwrap();
    assert_eq!(actual, expected);
    cleanup();
}

#[tokio::test]
#[cfg(not(ci))]
async fn should_return_not_found_if_not_exists() {
    init_db_folder();
    let expected = GetPreviewError::NotFound;
    let actual = get_file_preview(1).await.unwrap_err();
    assert_eq!(actual, expected);
    cleanup();
}
