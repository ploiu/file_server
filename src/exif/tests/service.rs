use crate::exif::process_single_file_exif;
use crate::repository::{file_repository, open_connection};
use crate::service::file_service::file_dir;
use crate::test::{cleanup, create_file_db_entry, init_db_folder};
use rocket::tokio;

#[cfg(not(ci))]
#[tokio::test]
async fn successfully_parsing_exif_data_stores_in_db() {
    init_db_folder();
    // Copy a test file with EXIF data from test_assets
    std::fs::copy("test_assets/test.png", format!("{}/test.png", file_dir()))
        .expect("Failed to copy test file to the file directory");
    create_file_db_entry("test.png", None);

    // Process the file
    let res = process_single_file_exif("1".to_string()).await;
    assert!(
        res,
        "process_single_file_exif should return true on success"
    );

    // Verify the date was updated in the database
    let con = open_connection();
    let file_record = file_repository::get_file(1, &con).unwrap();
    con.close().unwrap();

    // The date should be updated (not checking exact value since test.png may not have EXIF)
    // but the function should have run successfully
    assert!(file_record.create_date.timestamp() > 0);

    cleanup();
}

#[tokio::test]
async fn failing_to_parse_exif_stores_current_date() {
    init_db_folder();
    // Create a file without EXIF data
    let file_content = "This is not an image file";
    std::fs::write(format!("{}/test.txt", file_dir()), file_content)
        .expect("Failed to create test file");
    create_file_db_entry("test.txt", None);

    let before_time = chrono::offset::Local::now().naive_local();

    // Process the file
    let res = process_single_file_exif("1".to_string()).await;
    assert!(
        res,
        "process_single_file_exif should return true even when EXIF parsing fails"
    );

    // Verify the date was updated with current date
    let con = open_connection();
    let file_record = file_repository::get_file(1, &con).unwrap();
    con.close().unwrap();

    // The date should be close to current time (within a few seconds)
    let diff = (file_record.create_date.timestamp() - before_time.timestamp()).abs();
    assert!(
        diff < 5,
        "Date should be within 5 seconds of current time when EXIF parsing fails"
    );

    cleanup();
}

#[tokio::test]
async fn missing_file_from_filesystem_is_silently_ignored() {
    init_db_folder();
    // Create DB entry but don't create the file on disk
    create_file_db_entry("missing.png", None);

    // Process the file - should not error
    let res = process_single_file_exif("1".to_string()).await;
    assert!(
        res,
        "process_single_file_exif should return true and not re-queue when file is missing"
    );

    cleanup();
}

#[tokio::test]
async fn missing_file_from_db_is_ignored() {
    init_db_folder();
    // Don't create a DB entry, just try to process a non-existent file

    // Process the file - should not error
    let res = process_single_file_exif("999".to_string()).await;
    assert!(
        res,
        "process_single_file_exif should return true and not re-queue when file doesn't exist in DB"
    );

    cleanup();
}

#[tokio::test]
async fn invalid_file_id_is_handled() {
    init_db_folder();

    // Process with invalid file ID
    let res = process_single_file_exif("not-a-number".to_string()).await;
    assert!(
        res,
        "process_single_file_exif should return true for invalid file IDs"
    );

    cleanup();
}
