use std::path::Path;

use rocket::form::Form;
use rocket::http::Status;
use rocket::tokio::fs::create_dir;

use crate::model::request::file_upload::FileUpload;

static IMAGE_DIR: &str = "./images";


/// ensures that the passed directory exists on the file system
async fn check_image_dir(dir: &str) {
    let path = Path::new(dir);
    if !path.exists() {
        match create_dir(path).await {
            Ok(_) => (),
            Err(e) => panic!("Failed to create file directory: \n {:?}", e)
        }
    }
}

/// accepts a file via request body and stores it off
#[post("/", data = "<file_input>")]
pub async fn upload_file<>(mut file_input: Form<FileUpload<'_>>) -> Status {
    check_image_dir(IMAGE_DIR).await;
    let file_name = match file_input.file.name() {
        Some(name) => name,
        None => return Status::BadRequest
    };
    // create the file name from the parts
    let file_name = format!("{}/{}.{}", &IMAGE_DIR, file_name, file_input.extension);
    let path = Path::new(file_name.as_str());
    match file_input.file.persist_to(path).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            return Status::InternalServerError;
        }
    }
    return Status::NoContent;
}