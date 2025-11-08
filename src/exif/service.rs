use crate::model::file_types::FileTypes;
use crate::repository::{file_repository, open_connection};
use crate::service::file_service::{file_dir, get_file_path};
use chrono::NaiveDateTime;
use std::backtrace::Backtrace;
use std::path::PathBuf;

/// Attempts to parse EXIF data from a file and extract the original creation date.
/// 
/// ## Parameters
/// * `file_path` - The path to the file to parse
/// 
/// ## Returns
/// * `Some(NaiveDateTime)` if EXIF data was successfully parsed and contains a creation date
/// * `None` if EXIF parsing failed or no creation date was found
fn parse_exif_date(file_path: &str) -> Option<NaiveDateTime> {
    use std::io::Cursor;
    
    let file_data = match std::fs::read(file_path) {
        Ok(data) => data,
        Err(e) => {
            log::debug!("Failed to read file {file_path} for EXIF parsing: {e:?}");
            return None;
        }
    };

    let mut cursor = Cursor::new(&file_data);
    let exif_iter = match nom_exif::parse_exif(&mut cursor, None) {
        Ok(Some(exif)) => exif,
        Ok(None) => return None,
        Err(e) => {
            log::debug!("Failed to parse EXIF data for {file_path}: {e:?}");
            return None;
        }
    };

    // Look for DateTimeOriginal tag (0x9003)
    for entry in exif_iter {
        if entry.tag() == Some(nom_exif::ExifTag::DateTimeOriginal) {
            // Format entry using Debug and try to extract date string
            let debug_str = format!("{:?}", entry);
            // The debug format may contain the date, try to extract it
            // Try to find a date-like pattern in the output
            if let Some(date_str) = extract_date_from_debug(&debug_str) {
                if let Ok(dt) = NaiveDateTime::parse_from_str(&date_str, "%Y:%m:%d %H:%M:%S") {
                    return Some(dt);
                }
            }
        }
    }

    None
}

/// Helper function to extract date string from debug output
fn extract_date_from_debug(debug_str: &str) -> Option<String> {
    use regex::Regex;
    // Match EXIF date format: "YYYY:MM:DD HH:MM:SS"
    let re = Regex::new(r"\d{4}:\d{2}:\d{2} \d{2}:\d{2}:\d{2}").ok()?;
    re.find(debug_str).map(|m| m.as_str().to_string())
}

/// Processes a single file to extract EXIF data and update its creation date in the database.
/// 
/// ## Parameters
/// * `file_id` - The ID of the file to process
/// 
/// ## Returns
/// * `true` if the message should be removed from the queue (success or unrecoverable error)
/// * `false` if the message should be re-queued (temporary failure)
pub async fn process_single_file_exif(message_data: String) -> bool {
    let id: u32 = match message_data.parse() {
        Ok(i) => i,
        Err(e) => {
            log::error!(
                "Failed to parse {message_data} as a u32! Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            // Invalid message, don't re-queue
            return true;
        }
    };

    // Get file metadata from database
    let con = open_connection();
    let file_record = match file_repository::get_file(id, &con) {
        Ok(f) => f,
        Err(e) => {
            log::debug!(
                "Failed to get file from database when processing EXIF data. File id [{id}]; error is {e:?}",
            );
            con.close().unwrap();
            // File doesn't exist in DB, don't requeue
            return true;
        }
    };

    // Only process image and video files
    match file_record.file_type {
        FileTypes::Image | FileTypes::Video => {}
        _ => {
            con.close().unwrap();
            // Not an image/video, no EXIF to process
            return true;
        }
    }

    // Get file path on disk
    let path = match get_file_path(id) {
        Ok(p) => {
            let path = PathBuf::from(file_dir()).join(p);
            path.to_string_lossy().into_owned()
        }
        Err(_) => {
            log::debug!("File id {id} not found on disk, skipping EXIF processing");
            con.close().unwrap();
            // File is no longer on disk, don't requeue
            return true;
        }
    };

    // Parse EXIF date or use current date as fallback
    let create_date = parse_exif_date(&path).unwrap_or_else(|| chrono::offset::Local::now().naive_local());

    // Update file record in database with the extracted date
    let update_result = con.execute(
        "UPDATE FileRecords SET dateCreated = ?1 WHERE id = ?2",
        rusqlite::params![create_date, id],
    );

    con.close().unwrap();

    match update_result {
        Ok(_) => {
            log::debug!("Successfully updated creation date for file id {id}");
            true
        }
        Err(e) => {
            log::error!(
                "Failed to update creation date for file id {id}: {e:?}\n{}",
                Backtrace::force_capture()
            );
            // Database error, might be temporary, re-queue
            false
        }
    }
}

/// Queues all image and video files in the database for EXIF processing.
/// This function should be called on startup if the exif processing flag is not set.
pub fn mass_exif_process() {
    use crate::queue::publish_message;

    let con = open_connection();
    let all_files = match file_repository::get_all_files(&con) {
        Ok(files) => files,
        Err(e) => {
            log::error!(
                "Failed to retrieve files for mass EXIF processing: {e:?}\n{}",
                Backtrace::force_capture()
            );
            con.close().unwrap();
            return;
        }
    };
    con.close().unwrap();

    let mut queued_count = 0;
    for file in all_files {
        // Only queue image and video files
        match file.file_type {
            FileTypes::Image | FileTypes::Video => {
                if let Some(id) = file.id {
                    publish_message("exif_process", &id.to_string());
                    queued_count += 1;
                }
            }
            _ => continue,
        }
    }

    log::info!("Queued {queued_count} files for EXIF processing");
}
