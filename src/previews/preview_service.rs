use super::preview_dir;
use crate::model::error::file_errors::GetPreviewError;
use crate::model::file_types::FileTypes;
use crate::service::file_service::{self, file_dir};
use crate::{model::error::file_errors::GetFileError, service::file_service::get_file_path};
use rocket::tokio::fs;
use std::backtrace::Backtrace;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

/// checks if the ./file_previews directory exists. If not, it creates it.
///
/// panics:
/// panics if the file directory could not be created
pub fn ensure_preview_dir() {
    let dir_path = preview_dir();
    let path = Path::new(&dir_path);
    match std::fs::create_dir(path) {
        Ok(_) => {}
        Err(e) => {
            if e.kind() == ErrorKind::AlreadyExists {
                return;
            } else {
                panic!("Failed to create preview directory: {e:?}")
            }
        }
    }
    if !path.exists() {
        std::fs::create_dir(path).expect("Failed to create previews directory!");
    }
}

/// generates a preview of a file based on the passed message_data parameter.
/// Only files that are supported by ffmpeg will be converted
///
/// Will return `true` if the message can be acked by rabbit, `false` otherwise
///
/// ## Parameters
/// * [message_data] the rabbit message. Must correspond to a file id in the database
///
/// ## Returns
/// - false if the message should not be removed from the queue
/// - true if the message should be removed from the queue -
///   will happen under many circumstances, including bad messages, previews being successfully generated, no file with that id found, etc
pub async fn generate_preview(message_data: String) -> bool {
    ensure_preview_dir();
    if !check_ffmpeg() {
        // keep it in rabbit so that it can be retried later when ffmpeg is installed
        return false;
    }
    let id: u32 = match message_data.parse() {
        Ok(i) => i,
        Err(e) => {
            log::error!(
                "Failed to parse {message_data} as a u32! Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            // we can't re-queue this or else we'll keep getting errors
            return true;
        }
    };
    // we can check to see if the file is even a type we can generate a preview for first
    let file_data = match file_service::get_file_metadata(id) {
        Ok(f) => f,
        Err(e) => {
            log::warn!(
                "Failed to get file from database when generating preview! Was the file deleted? File id [{id}]; error is {e:?}",
            );
            // file doesn't exist, don't requeue it
            return true;
        }
    };
    let path = match get_file_path(id) {
        Ok(p) => {
            let path = PathBuf::from(file_dir()).join(p);
            path.to_string_lossy().into_owned()
        }
        Err(GetFileError::NotFound) => {
            // file is no longer on disk, meaning it was deleted
            return true;
        }
        Err(e) => {
            log::error!(
                "Failed to get file path for file id {id}. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            // TODO maybe limit the number of times a file can be re-acked? Until then, we can't re-queue
            return true;
        }
    };
    let preview_path = preview_dir();
    let output_file_name = format!("{id}.png");
    let output_path = PathBuf::from(&preview_path).join(&output_file_name);
    
    // Check if preview already exists
    if output_path.exists() {
        log::debug!("Preview already exists for file id [{id}], skipping generation");
        return true;
    }
    
    let mut command = Command::new("ffmpeg");
    let output_path = output_path.to_string_lossy();
    if Some(FileTypes::Image) == file_data.file_type
        && !file_data.name.to_lowercase().ends_with(".gif")
    {
        command.args(["-i", &path, "-vf", "scale=150:-1", &output_path]);
    } else if Some(FileTypes::Video) == file_data.file_type
        || file_data.name.to_lowercase().ends_with(".gif")
    {
        command.args([
            "-i",
            &path,
            "-vf",
            "scale=150:-1",
            "-frames:v",
            "1",
            &output_path,
        ]);
    } else {
        // invalid file type
        return true;
    };
    log::debug!("running ffmpeg: {command:?}");
    let output = match command.output() {
        Ok(o) => o,
        Err(e) => {
            log::error!(
                "Catastrophic error trying to execute ffmpeg after it was already checked!: {e:?}\n{}",
                Backtrace::force_capture()
            );
            return true;
        }
    };
    if !output.status.success() {
        if cfg!(test) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("ffmpeg stderr: {stderr}");
        }
        log::warn!(
            "Failed to perform ffmpeg conversion for file with id [{id}]. Status code is {:?}",
            output.status.code()
        );
    }
    true
}

/// Retrieves the preview contents of the file with the passed id in png format.
/// The preview might not immediately exist in the database at the time this function is called,
/// so extra care needs to be taken to not blow up if (when) that happens.
///
/// # Errors
///
/// This function will return an error if the preview doesn't exist in the database, or if the database fails. Regardless, a log will be emitted
pub async fn get_file_preview(id: u32) -> Result<Vec<u8>, GetPreviewError> {
    let preview_path = format!("{}/{id}.png", preview_dir());
    match fs::read(&preview_path).await {
        Ok(contents) => Ok(contents),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(GetPreviewError::NotFound)
            } else {
                log::error!(
                    "Failed to read preview file at path {preview_path}! Exception is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                Err(GetPreviewError::FileSystemError)
            }
        }
    }
}

/// deletes the preview file for the file with the passed id
///
/// # Parameters
/// * `id` - the id of the file whose preview should be deleted
///
/// If the file doesn't exist, a warning will be logged but no error will be thrown
pub fn delete_file_preview(id: u32) {
    let preview_path = preview_dir();
    let file_path = format!("{preview_path}/{id}.png");
    let path = Path::new(&file_path);
    match std::fs::remove_file(path) {
        Ok(_) => {}
        Err(e) => {
            log::warn!(
                "Failed to delete preview file at path {file_path}! Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
        }
    }
}

/// retrieves all file IDs from the database and publishes them to the preview generation queue.
/// This function is intended to be called in a background thread as it may take some time
/// to publish all messages depending on the number of files.
pub fn load_all_files_in_preview_queue() {
    use crate::repository::{file_repository, open_connection};

    let con = open_connection();
    let file_ids = match file_repository::get_all_file_ids(&con) {
        Ok(ids) => ids,
        Err(e) => {
            con.close().unwrap_or(());
            log::error!(
                "Failed to retrieve file IDs for preview regeneration: {e:?}\n{}",
                Backtrace::force_capture()
            );
            return;
        }
    };

    log::debug!("Publishing {} file IDs to preview queue", file_ids.len());

    for id in file_ids {
        crate::queue::publish_message("icon_gen", &id.to_string());
    }

    con.close().unwrap_or(());
    log::debug!("Successfully published all file IDs to preview queue");
}

/// checks if ffmpeg is installed on the system
fn check_ffmpeg() -> bool {
    let output = Command::new("ffmpeg")
        .arg("-h")
        .output()
        .expect("Failed to check if ffmpeg is installed due to a fatal error!");
    let code = output.status.code();
    if output.status.success() {
        true
    } else {
        log::error!("ffmpeg is not installed. Check returned with code {code:?}");
        false
    }
}
