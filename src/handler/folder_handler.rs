use rocket::serde::json::Json;

use crate::guard::HeaderAuth;
use crate::model::error::folder_errors::{
    CreateFolderError, DeleteFolderError, GetFolderError, UpdateFolderError,
};
use crate::model::guard::auth::ValidateResult;
use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
use crate::model::response::folder_responses::{
    CreateFolderResponse, DeleteFolderResponse, GetFolderResponse, UpdateFolderResponse,
};
use crate::model::response::BasicMessage;
use crate::service::folder_service;

#[get("/<id>")]
pub fn get_folder(id: Option<u32>, auth: HeaderAuth) -> GetFolderResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return GetFolderResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetFolderResponse::Unauthorized("Bad Credentials".to_string())
    };
    match folder_service::get_folder(id) {
        Ok(folder) => GetFolderResponse::Success(Json::from(folder)),
        Err(GetFolderError::NotFound) => GetFolderResponse::FolderNotFound(BasicMessage::new(
            "The folder with the passed id could not be found.",
        )),
        // TODO maybe distinguish between not found on disk and not able to pull in DB?
        Err(_) => GetFolderResponse::FolderDbError(BasicMessage::new(
            "Failed to pull folder info from database. Check server logs for details",
        )),
    }
}

#[post("/", data = "<folder>")]
pub async fn create_folder(
    folder: Json<CreateFolderRequest>,
    auth: HeaderAuth,
) -> CreateFolderResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return CreateFolderResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return CreateFolderResponse::Unauthorized("Bad Credentials".to_string())
    };
    match folder_service::create_folder(&folder.into_inner()).await {
        Ok(f) => CreateFolderResponse::Success(Json::from(f)),
        Err(CreateFolderError::ParentNotFound) => CreateFolderResponse::ParentNotFound(
            BasicMessage::new("No folder with the passed parentId was found."),
        ),
        Err(CreateFolderError::AlreadyExists) => CreateFolderResponse::FolderAlreadyExists(
            BasicMessage::new("That folder already exists."),
        ),
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
    }
}

#[put("/", data = "<folder>")]
pub fn update_folder(folder: Json<UpdateFolderRequest>, auth: HeaderAuth) -> UpdateFolderResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return UpdateFolderResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return UpdateFolderResponse::Unauthorized("Bad Credentials".to_string())
    };
    match folder_service::update_folder(&folder) {
        Ok(updated_folder) => UpdateFolderResponse::Success(Json::from(updated_folder)),
        Err(UpdateFolderError::NotFound) => UpdateFolderResponse::FolderNotFound(BasicMessage::new("The folder with the passed id could not be found.")),
        Err(UpdateFolderError::ParentNotFound) => UpdateFolderResponse::ParentNotFound(BasicMessage::new("The parent folder with the passed id could not be found.")),
        Err(UpdateFolderError::AlreadyExists) => UpdateFolderResponse::FolderAlreadyExists(BasicMessage::new("Cannot update folder, because another one with the new path already exists.")),
        Err(UpdateFolderError::FileAlreadyExists) => UpdateFolderResponse::FolderAlreadyExists(BasicMessage::new("A file with that name already exists.")),
        Err(UpdateFolderError::DbFailure) => UpdateFolderResponse::FolderDbError(BasicMessage::new("Could not update the folder in the database. Please check the server logs for more details.")),
        Err(UpdateFolderError::FileSystemFailure) => UpdateFolderResponse::FileSystemError(BasicMessage::new("Could not move the folder! Please see server logs for details.")),
        Err(UpdateFolderError::NotAllowed) => UpdateFolderResponse::FolderAlreadyExists(BasicMessage::new("Cannot move parent folder into its own child.")),
        Err(UpdateFolderError::TagError) => UpdateFolderResponse::TagError(BasicMessage::new("Failed to update tags. Check server logs for details.")),
    }
}

#[delete("/<id>")]
pub fn delete_folder(id: u32, auth: HeaderAuth) -> DeleteFolderResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return DeleteFolderResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DeleteFolderResponse::Unauthorized("Bad Credentials".to_string())
    };
    match folder_service::delete_folder(id) {
        Ok(()) => DeleteFolderResponse::Success(()),
        Err(DeleteFolderError::FolderNotFound) => DeleteFolderResponse::FolderNotFound(BasicMessage::new("The folder with the request id does not exist.")),
        Err(DeleteFolderError::DbFailure) => DeleteFolderResponse::FolderDbError(BasicMessage::new("Failed to remove folder reference from the database. Check server logs for details.")),
        Err(DeleteFolderError::FileSystemError) => DeleteFolderResponse::FileSystemError(BasicMessage::new("Failed to remove folder from the file system. Check server logs for details."))
    }
}
