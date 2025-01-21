use std::backtrace::Backtrace;
use std::io::Cursor;

use image::DynamicImage;
use image::ImageReader;
use rusqlite::Connection;

use crate::model::error::preview_errors::PreviewError;
use crate::repository::{file_repository, open_connection};
use crate::{model::error::file_errors::GetFileError, service::file_service::get_file_path};

/// generates a preview of a file based on the passed `message_data` parameter.
/// Not all files will have previews generated (mainly images, videos, and gifs)
///
/// Will return a `true` if the message can be acked by rabbit, and `false` if the message needs to be re-queued
///
/// * [message_data] the rabbit message. Must be a number that fits in u32
pub async fn generate_preview(message_data: String) -> bool {
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
    // 1) get file path
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
    let preview_blob: Vec<u8>;
    if let Ok(blob) = resize_image(&path) {
        preview_blob = blob;
    } else {
        // none of these errors are really recoverable...
        return true;
    }
    // now time to store our blob in the database
    let con: Connection = open_connection();
    let create_result = file_repository::create_file_preview(id, preview_blob, &con);
    con.close().unwrap();
    if let Err(e) = create_result {
        log::error!(
            "Failed to save file preview in the database for file id {id}. Exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        // TODO really we would benefit from the ability to try twice or something...
        return true;
    }
    true
}

/// Generates a list of bytes for an image, sized down to at most 300x300 pixels.
/// This byte list is stored in the png format
///
/// # Errors
///
/// This function will return an error if the underlying [image] library returns any errors.
/// This will mostly happen with reading, encoding, decoding, or resizing the image
///
/// # Params
///
/// * [image_path] the full file path to the image, relative to where this program is running
fn resize_image(image_path: &str) -> Result<Vec<u8>, PreviewError> {
    let img = match ImageReader::open(image_path) {
        Ok(i) => i,
        Err(e) => {
            log::error!(
                "Failed to open image at path {image_path}. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(PreviewError::Open);
        }
    };
    // this gives better insight if the file extension is wrong
    let img = match img.with_guessed_format() {
        Ok(i) => i,
        Err(e) => {
            log::error!(
                "Failed to guess the format of path {image_path}. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(PreviewError::GuessFormat);
        }
    };
    let img: DynamicImage = match img.decode() {
        Ok(i) => i,
        Err(e) => {
            log::error!("Failed to decode the image at path {image_path}. Exception is {e:?}");
            return Err(PreviewError::Decode);
        }
    };
    let resized = img.resize(150, 150, image::imageops::FilterType::Gaussian);
    let mut blob = Vec::<u8>::new();
    if let Err(e) = resized.write_to(&mut Cursor::new(&mut blob), image::ImageFormat::Png) {
        log::error!("Failed to write resized image with path [{image_path}] to blob array. Exception is {e:?}\n{}", Backtrace::force_capture());
        return Err(PreviewError::Encode);
    }
    Ok(blob)
}
