use crate::model::api::FileApi;
use crate::model::error::file_errors::GetPreviewError;
use crate::model::error::preview_errors::PreviewError;
use crate::model::file_types::FileTypes;
use crate::repository::open_connection;
use crate::service::file_service;
use crate::{
    model::error::file_errors::GetFileError, repository, service::file_service::get_file_path,
};
use image::DynamicImage;
use image::ImageReader;
use rocket::tokio::fs::create_dir;
use rusqlite::Connection;
use std::backtrace::Backtrace;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;

#[cfg(not(test))]
fn preview_dir() -> String {
    "./file_previews".to_string()
}

#[cfg(test)]
pub fn preview_dir() -> String {
    let thread_name = crate::test::current_thread_name();
    let dir_name = format!("./{thread_name}_previews");
    dir_name
}

/// checks if the ./file_previews directory exists. If not, it creates it.
///
/// panics:
/// panics if the file directory could not be created
pub fn ensure_preview_dir() {
    let dir_path = preview_dir();
    let path = Path::new(&dir_path);
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
pub async fn generate_preview(message_data: String) -> bool {
    ensure_preview_dir();
    if !check_ffmpeg() {
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
                "Failed to get file from database when generating preview! Was the file deleted? File id [{id}]"
            );
            // file doesn't exist, don't requeue it
            return true;
        }
    };
    let path = match get_file_path(id) {
        Ok(p) => format!("./files/{p}"),
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
    let output_file_name = format!("./{id}.png");
    let command = if Some(FileTypes::Image) == file_data.file_type {
        format!("ffmpeg -i {path} -vf scale=150:-1 {preview_path}/{output_file_name}")
    } else if Some(FileTypes::Video) == file_data.file_type {
        format!("ffmpeg -i {path} -vf scale=150:-1 -frames:v 1 {preview_path}/{output_file_name}")
    } else {
        // invalid file type
        return true;
    };
    let output = match Command::new("sh").arg("-c").arg(&command).output() {
        Ok(o) => o.status,
        Err(e) => {
            log::error!(
                "Catastrophic error trying to execute ffmpeg after it was already checked!: {e:?}\n{}",
                Backtrace::force_capture()
            );
            return true;
        }
    };
    if !output.success() {
        log::warn!(
            "Failed to perform ffmpeg conversion for file with id [{id}]. Status code is {:?}",
            output.code()
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
pub fn get_file_preview(id: u32) -> Result<Vec<u8>, GetPreviewError> {
    let preview_path = format!("{}/{id}.png", preview_dir());
    let path = Path::new(&preview_path);
    if !path.exists() {
        return Err(GetPreviewError::NotFound);
    }
    // we know the file exists, so now we can return the contents
    match std::fs::read(&preview_path) {
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

/// checks if ffmpeg is installed on the system
fn check_ffmpeg() -> bool {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ffmpeg -h")
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

#[cfg(test)]
mod generate_preview_tests {}

#[cfg(test)]
mod get_file_preview_tests {}

#[cfg(test)]
mod delete_file_preview_tests {
    use super::{delete_file_preview, preview_dir};
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
}
