use std::fs::File;
use std::path::Path;

use crate::facade::db::check_auth;
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::status;
use rocket::tokio::fs::create_dir;

use crate::facade::db::save_file_record;
use crate::guard::Auth;
use crate::model::request::FileUpload;
use crate::model::response::BasicResponse;

static FILE_DIR: &str = "./files";

/// ensures that the passed directory exists on the file system
async fn check_image_dir(dir: &str) {
    let path = Path::new(dir);
    if !path.exists() {
        match create_dir(path).await {
            Ok(_) => (),
            Err(e) => panic!("Failed to create file directory: \n {:?}", e),
        }
    }
}

/// accepts a file via request body and stores it off
#[post("/", data = "<file_input>")]
pub async fn upload_file(
    mut file_input: Form<FileUpload<'_>>,
    auth: Auth,
) -> (Status, BasicResponse<'_>) {
    match auth.validate() {
        Some(v) => return v,
        _ => {}
    };
    check_image_dir(FILE_DIR).await;
    let file_name = match file_input.file.name() {
        Some(name) => name,
        None => return BasicResponse::text(Status::BadRequest, "file name is required"),
    };
    // create the file name from the parts
    let file_name = format!("{}/{}.{}", &FILE_DIR, file_name, file_input.extension);
    let path = Path::new(file_name.as_str());
    match file_input.file.persist_to(path).await {
        Ok(_) => {
            // since this is guaranteed to happen after the file is successfully saved, we can unwrap here
            let mut saved_file = File::open(path).unwrap();
            match save_file_record(&file_name, &path, &mut saved_file) {
                Err(e) => {
                    eprintln!("Failed to create file record in database: {:?}", e);
                    return BasicResponse::text(
                        Status::InternalServerError,
                        "failed to save record to database",
                    );
                }
                _ => {}
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            return BasicResponse::text(Status::InternalServerError, "failed to save file to disk");
        }
    }
    return BasicResponse::text(Status::NoContent, "");
}
