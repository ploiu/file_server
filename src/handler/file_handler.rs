use rocket::form::Form;

use crate::guard::Auth;
use crate::model::request::FileUpload;
use crate::model::response::file_responses::{CreateFileResponse, GetFileResponse};
use crate::service::file_service;
use crate::service::file_service::{save_file, GetFileError, SaveFileError};

/// accepts a file via request body and stores it off
#[post("/", data = "<file_input>")]
pub async fn upload_file(file_input: Form<FileUpload<'_>>, auth: Auth) -> CreateFileResponse {
    if !auth.validate() {
        return CreateFileResponse::Unauthorized("Invalid Credentials".to_string());
    }
    return match save_file(&mut file_input.into_inner()).await {
        Ok(_) => CreateFileResponse::Created(()),
        Err(e) => match e {
            SaveFileError::MissingInfo(message) => {
                CreateFileResponse::BadRequest(format!("{{\"message\": \"{:?}\"}}", message))
            }
            //language=json
            SaveFileError::FailWriteDisk => CreateFileResponse::Failure(
                "{\"message\": \"Failed to save file to disk!\"}".to_string(),
            ),
            //language=json
            SaveFileError::FailWriteDb => CreateFileResponse::Failure(
                "{\"message\": \"Failed to save file info to database!\"}".to_string(),
            ),
        },
    };
}

#[get("/<id>")]
pub async fn get_file(id: u64, auth: Auth) -> GetFileResponse {
    if !auth.validate() {
        return GetFileResponse::Unauthorized("Invalid Credentials".to_string());
    }
    return match file_service::get_file(id) {
        Ok(file) => GetFileResponse::Success(file),
        //language=json
        Err(message) if message == GetFileError::NotFound => GetFileResponse::FileNotFound(
            "{\"message\": \"The file with the passed id could not be found.\"}".to_string(),
        ),
        //language=json TODO maybe distinguish between not found on disk and not able to pull in DB?
        Err(_) => GetFileResponse::FileDbError("{\"message\": \"Failed to pull file info from database. Check server logs for details\"}".to_string())
    };
}
