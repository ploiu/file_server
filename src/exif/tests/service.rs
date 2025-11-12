use crate::exif::process_single_file_exif;
use crate::model::file_types::FileTypes;
use crate::model::repository::FileRecord;
use crate::repository::{file_repository, open_connection};
use crate::service::file_service::file_dir;
use crate::test::{cleanup, create_file_disk, init_db_folder};
use rocket::tokio;

#[cfg(not(ci))]
#[tokio::test]
async fn successfully_parsing_exif_data_stores_in_db() {
    init_db_folder();
    // Copy a test file with EXIF data from test_assets
    std::fs::copy("test_assets/test.png", format!("{}/test.png", file_dir()))
        .expect("Failed to copy test file to the file directory");

    // Create a file record with a hard-coded old date
    let old_date =
        chrono::NaiveDateTime::parse_from_str("0001-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let file_record = FileRecord {
        id: None,
        name: "test.png".to_string(),
        parent_id: None,
        create_date: old_date,
        size: 100,
        file_type: FileTypes::Image,
    }
    .save_to_db();

    let file_id = file_record.id.unwrap();

    // Verify the initial date
    let con = open_connection();
    let initial_record = file_repository::get_file(file_id, &con).unwrap();
    assert_eq!(
        initial_record.create_date, old_date,
        "Initial date should be the old date"
    );

    // Process the file
    let res = process_single_file_exif(file_id.to_string()).await;
    assert!(
        res,
        "process_single_file_exif should return true on success"
    );

    // Verify the date was updated to something greater than the old date
    let updated_record = file_repository::get_file(file_id, &con).unwrap();
    con.close().unwrap();

    assert!(
        updated_record.create_date > old_date,
        "Date should be updated to be greater than the old date"
    );

    cleanup();
}

#[tokio::test]
async fn failing_to_parse_exif_stores_current_date() {
    init_db_folder();
    // Create a file without EXIF data - use an image extension but with non-image content
    let file_content = "This is not an image file";
    create_file_disk("test.jpg", file_content);

    // Create a file record with a hard-coded old date way in the past
    let old_date =
        chrono::NaiveDateTime::parse_from_str("1900-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let file_record = FileRecord {
        id: None,
        name: "test.jpg".to_string(),
        parent_id: None,
        create_date: old_date,
        size: file_content.len() as u64,
        file_type: FileTypes::Image,
    }
    .save_to_db();

    let file_id = file_record.id.unwrap();
    let before_time = chrono::offset::Local::now().naive_local();

    // Process the file
    let res = process_single_file_exif(file_id.to_string()).await;
    assert!(
        res,
        "process_single_file_exif should return true even when EXIF parsing fails"
    );

    // Verify the date was updated with current date
    let con = open_connection();
    let updated_record = file_repository::get_file(file_id, &con).unwrap();
    con.close().unwrap();

    // The date should be close to current time (within a few seconds)
    let diff = (updated_record.create_date.timestamp() - before_time.timestamp()).abs();
    assert!(
        diff < 5,
        "Date should be within 5 seconds of current time when EXIF parsing fails"
    );

    // Also verify it's much greater than the old date
    assert!(
        updated_record.create_date > old_date,
        "Updated date should be much greater than the old date from 1900"
    );

    cleanup();
}

#[tokio::test]
async fn missing_file_from_filesystem_is_silently_ignored() {
    init_db_folder();
    // Create a file record with an old date but don't create the file on disk
    let old_date =
        chrono::NaiveDateTime::parse_from_str("1900-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let file_record = FileRecord {
        id: None,
        name: "missing.png".to_string(),
        parent_id: None,
        create_date: old_date,
        size: 100,
        file_type: FileTypes::Image,
    }
    .save_to_db();

    let file_id = file_record.id.unwrap();

    // Process the file - should not error
    let res = process_single_file_exif(file_id.to_string()).await;
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
