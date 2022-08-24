use crate::guard::{Auth, ValidateResult};
use crate::model::request::folder_requests::CreateFolderRequest;
use crate::model::response::folder_responses::{CreateFolderResponse, GetFolderResponse};
use crate::model::response::BasicMessage;
use crate::service::folder_service;
use crate::service::folder_service::{CreateFolderError, GetFolderError};
use rocket::serde::json::Json;

#[get("/<id>")]
pub fn get_folder(id: u32, auth: Auth) -> GetFolderResponse {
    match auth.validate() {
        ValidateResult::Ok => {/*no op*/}
        ValidateResult::NoPasswordSet => return GetFolderResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetFolderResponse::Unauthorized("Bad Credentials".to_string())
    };
    return match folder_service::get_folder(id) {
        Ok(folder) => GetFolderResponse::Success(Json::from(folder)),
        Err(message) if message == GetFolderError::NotFound => GetFolderResponse::FolderNotFound(
            BasicMessage::new("The folder with the passed id could not be found."),
        ),
        // TODO maybe distinguish between not found on disk and not able to pull in DB?
        Err(_) => GetFolderResponse::FolderDbError(BasicMessage::new(
            "Failed to pull folder info from database. Check server logs for details",
        )),
    };
}

#[post("/", data = "<folder>")]
pub fn create_folder(folder: Json<CreateFolderRequest>, auth: Auth) -> CreateFolderResponse {
    match auth.validate() {
        ValidateResult::Ok => {/*no op*/}
        ValidateResult::NoPasswordSet => return CreateFolderResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return CreateFolderResponse::Unauthorized("Bad Credentials".to_string())
    };
    return match folder_service::create_folder(&folder.into_inner()) {
        Ok(f) => CreateFolderResponse::Success(Json::from(f)),
        Err(message) if message == CreateFolderError::ParentNotFound => {
            CreateFolderResponse::ParentNotFound(BasicMessage::new(
                "No folder with the passed parentId was found",
            ))
        }
        Err(e) if e == CreateFolderError::AlreadyExists => {
            CreateFolderResponse::FolderAlreadyExists(BasicMessage::new(
                "That folder already exists",
            ))
        }
        Err(e) if e == CreateFolderError::FileSystemFailure => {
            eprintln!(
                "Failed to save folder to disk! Nested exception is: \n{:?}",
                e
            );
            CreateFolderResponse::FileSystemError(BasicMessage::new(
                "Failed to save folder to the file system. See server logs for details.",
            ))
        }
        Err(e) => {
            eprintln!("failed to save folder, nested exception is:\n {:?}", e);
            CreateFolderResponse::FolderDbError(BasicMessage::new(
                "Failed to save folder info to the database. Check server logs for details",
            ))
        }
    };
}
