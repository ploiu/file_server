use rocket::form::Form;
use rocket::serde::json::Json;

use crate::guard::{Auth, ValidateResult};
use crate::model::request::file_requests::CreateFileRequest;
use crate::model::response::file_responses::{
    CreateFileResponse, DeleteFileResponse, GetFileResponse,
};
use crate::model::response::BasicMessage;
use crate::service::file_service;
use crate::service::file_service::{save_file, DeleteFileError, GetFileError, SaveFileError};

/// accepts a file via request body and stores it off
#[post("/", data = "<file_input>")]
pub async fn upload_file(
    file_input: Form<CreateFileRequest<'_>>,
    auth: Auth,
) -> CreateFileResponse {
    match auth.validate() {
        ValidateResult::Ok => {/*no op*/}
        ValidateResult::NoPasswordSet => return CreateFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return CreateFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    return match save_file(&mut file_input.into_inner()).await {
        Ok(f) => CreateFileResponse::Success(Json::from(f)),
        Err(e) => match e {
            SaveFileError::MissingInfo(message) => {
                CreateFileResponse::BadRequest(BasicMessage::new(message.as_str()))
            }
            SaveFileError::FailWriteDisk => {
                CreateFileResponse::Failure(BasicMessage::new("Failed to save file to disk!"))
            }
            SaveFileError::FailWriteDb => CreateFileResponse::Failure(BasicMessage::new(
                "Failed to save file info to database!",
            )),
            SaveFileError::ParentFolderNotFound => CreateFileResponse::NotFound(BasicMessage::new(
                "No parent folder with the passed id was found",
            )),
        },
    };
}

#[get("/<id>")]
pub async fn get_file(id: u32, auth: Auth) -> GetFileResponse {
    match auth.validate() {
        ValidateResult::Ok => {/*no op*/}
        ValidateResult::NoPasswordSet => return GetFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    return match file_service::get_file(id) {
        Ok(file) => GetFileResponse::Success(file),
        Err(message) if message == GetFileError::NotFound => GetFileResponse::FileNotFound(
            BasicMessage::new("The file with the passed id could not be found."),
        ),
        // TODO maybe distinguish between not found on disk and not able to pull in DB?
        Err(_) => GetFileResponse::FileDbError(BasicMessage::new(
            "Failed to pull file info from database. Check server logs for details",
        )),
    };
}

#[delete("/<id>")]
pub fn delete_file(id: u32, auth: Auth) -> DeleteFileResponse {
    match auth.validate() {
        ValidateResult::Ok => {/*no op*/}
        ValidateResult::NoPasswordSet => return DeleteFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DeleteFileResponse::Unauthorized("Bad Credentials".to_string())
    };
    return match file_service::delete_file(id) {
        Ok(()) => DeleteFileResponse::Deleted(()),
        Err(e) if e == DeleteFileError::NotFound => {
            DeleteFileResponse::NotFound(BasicMessage::new("No file with the passed id was found."))
        }
        Err(e) if e == DeleteFileError::DbError => DeleteFileResponse::Failure(BasicMessage::new(
            "Failed to remove file reference from database.",
        )),
        Err(e) if e == DeleteFileError::FileSystemError => DeleteFileResponse::Failure(
            BasicMessage::new("Failed to remove file from the file system."),
        ),
        _ => panic!("delete file - we shouldn't reach here!"),
    };
}
