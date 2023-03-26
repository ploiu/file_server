use rocket::form::Form;
use rocket::serde::json::Json;

use crate::guard::Auth;
use crate::model::error::file_errors::{
    CreateFileError, DeleteFileError, GetFileError, SearchFileError, UpdateFileError,
};
use crate::model::guard::auth::ValidateResult;
use crate::model::request::file_requests::{CreateFileRequest, UpdateFileRequest};
use crate::model::response::file_responses::{
    CreateFileResponse, DeleteFileResponse, DownloadFileResponse, FileMetadataResponse,
    GetFileResponse, SearchFileResponse, UpdateFileResponse,
};
use crate::model::response::BasicMessage;
use crate::service::file_service;
use crate::service::file_service::save_file;

/// accepts a file via request body and stores it off
#[post("/", data = "<file_input>")]
pub async fn upload_file(
    file_input: Form<CreateFileRequest<'_>>,
    auth: Auth,
) -> CreateFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return CreateFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return CreateFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    return match save_file(&mut file_input.into_inner()).await {
        Ok(f) => CreateFileResponse::Success(Json::from(f)),
        Err(e) => match e {
            CreateFileError::MissingInfo(message) => {
                CreateFileResponse::BadRequest(BasicMessage::new(message.as_str()))
            }
            CreateFileError::FailWriteDisk => {
                CreateFileResponse::Failure(BasicMessage::new("Failed to save file to disk!"))
            }
            CreateFileError::FailWriteDb => CreateFileResponse::Failure(BasicMessage::new(
                "Failed to save file info to database!",
            )),
            CreateFileError::ParentFolderNotFound => CreateFileResponse::NotFound(
                BasicMessage::new("No parent folder with the passed id was found"),
            ),
            CreateFileError::AlreadyExists => {
                CreateFileResponse::AlreadyExists(BasicMessage::new("That file already exists"))
            }
        },
    };
}

#[get("/metadata/<id>")]
pub fn get_file(id: u32, auth: Auth) -> GetFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return GetFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    return match file_service::get_file_metadata(id) {
        Ok(file) => GetFileResponse::Success(Json::from(FileMetadataResponse::from(&file))),
        Err(message) if message == GetFileError::NotFound => GetFileResponse::FileNotFound(
            BasicMessage::new("The file with the passed id could not be found."),
        ),
        // TODO maybe distinguish between not found on disk and not able to pull in DB?
        Err(_) => GetFileResponse::FileDbError(BasicMessage::new(
            "Failed to pull file info from database. Check server logs for details",
        )),
    };
}

#[get("/metadata?<search>")]
pub fn search_files(search: String, auth: Auth) -> SearchFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return SearchFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return SearchFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    if search.trim().is_empty() {
        return SearchFileResponse::BadRequest(BasicMessage::new("Search string is required."));
    }
    return match file_service::search_files(search) {
        Ok(files) => SearchFileResponse::Success(Json::from(files)),
        Err(SearchFileError::DbError) => SearchFileResponse::GenericError(BasicMessage::new(
            "Failed to search files. Check server logs for details",
        )),
    };
}

#[get("/<id>")]
pub fn download_file(id: u32, auth: Auth) -> DownloadFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return DownloadFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DownloadFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    return match file_service::get_file_contents(id) {
        Ok(f) => DownloadFileResponse::Success(f),
        Err(e) if e == GetFileError::NotFound => DownloadFileResponse::FileNotFound(BasicMessage::new("The file with the passed id could not be found.")),
        Err(e) if e == GetFileError::DbFailure => DownloadFileResponse::FileDbError(BasicMessage::new("Failed to retrieve the file info from the database. Check the server logs for details")),
        Err(_) => panic!("Download file: We should never get here")
    };
}

#[delete("/<id>")]
pub fn delete_file(id: u32, auth: Auth) -> DeleteFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return DeleteFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DeleteFileResponse::Unauthorized("Bad Credentials".to_string())
    };
    return match file_service::delete_file(id) {
        Ok(()) => DeleteFileResponse::Deleted(()),
        Err(e) if e == DeleteFileError::NotFound => DeleteFileResponse::NotFound(
            BasicMessage::new("The file with the passed id could not be found."),
        ),
        Err(e) if e == DeleteFileError::DbError => DeleteFileResponse::Failure(BasicMessage::new(
            "Failed to remove file reference from database.",
        )),
        Err(e) if e == DeleteFileError::FileSystemError => DeleteFileResponse::Failure(
            BasicMessage::new("Failed to remove file from the file system."),
        ),
        _ => panic!("delete file - we shouldn't reach here!"),
    };
}

#[put("/", data = "<data>")]
pub async fn update_file(data: Json<UpdateFileRequest>, auth: Auth) -> UpdateFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return UpdateFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return UpdateFileResponse::Unauthorized("Bad Credentials".to_string())
    };

    return match file_service::update_file(data.into_inner()).await {
        Ok(f) => UpdateFileResponse::Success(Json::from(f)),
        Err(e) if e == UpdateFileError::NotFound => UpdateFileResponse::NotFound(
            BasicMessage::new("The file with the passed id could not be found."),
        ),
        Err(e) if e == UpdateFileError::FolderNotFound => UpdateFileResponse::NotFound(
            BasicMessage::new("The folder with the passed id could not be found."),
        ),
        Err(e) if e == UpdateFileError::FileAlreadyExists => UpdateFileResponse::BadRequest(
            BasicMessage::new("A file with the same name already exists in the specified folder"),
        ),
        Err(_) => UpdateFileResponse::GenericError(BasicMessage::new(
            "Failed to update the file. Check the server logs for details",
        )),
    };
}
