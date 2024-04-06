use rocket::form::{Form, Strict};
use rocket::serde::json::Json;

use crate::guard::HeaderAuth;
use crate::model::api::FileApi;
use crate::model::error::file_errors::{
    CreateFileError, DeleteFileError, GetFileError, GetPreviewError, SearchFileError,
    UpdateFileError,
};
use crate::model::guard::auth::ValidateResult;
use crate::model::request::file_requests::CreateFileRequest;
use crate::model::response::file_responses::{
    CreateFileResponse, DeleteFileResponse, DownloadFileResponse, GetFileResponse,
    GetPreviewResponse, SearchFileResponse, UpdateFileResponse,
};
use crate::model::response::BasicMessage;
use crate::service::file_service::save_file;
use crate::service::{file_service, preview_service, search_service};

/// accepts a file via request body and stores it off
#[post("/?<force>", data = "<file_input>")]
pub async fn upload_file(
    file_input: Form<Strict<CreateFileRequest<'_>>>,
    force: Option<bool>,
    auth: HeaderAuth,
) -> CreateFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return CreateFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return CreateFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    match save_file(&mut file_input.into_inner(), force.unwrap_or(false)).await {
        Ok(f) => CreateFileResponse::Success(Json::from(f)),
        Err(e) => match e {
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
    }
}

#[get("/metadata/<id>")]
pub fn get_file(id: u32, auth: HeaderAuth) -> GetFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return GetFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    match file_service::get_file_metadata(id) {
        Ok(file) => GetFileResponse::Success(Json::from(file)),
        Err(GetFileError::NotFound) => GetFileResponse::FileNotFound(BasicMessage::new(
            "The file with the passed id could not be found.",
        )),
        // TODO maybe distinguish between not found on disk and not able to pull in DB?
        Err(_) => GetFileResponse::FileDbError(BasicMessage::new(
            "Failed to pull file info from database. Check server logs for details",
        )),
    }
}

#[get("/metadata?<search>&<tags>")]
pub fn search_files(
    search: Option<String>,
    tags: Option<Vec<String>>,
    auth: HeaderAuth,
) -> SearchFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return SearchFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return SearchFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    let search = search.unwrap_or("".to_string());
    let tags = tags.unwrap_or_default();
    if search.is_empty() && tags.is_empty() {
        return SearchFileResponse::BadRequest(BasicMessage::new(
            "Search string or tags are required.",
        ));
    }
    match search_service::search_files(search, tags) {
        Ok(files) => {
            SearchFileResponse::Success(Json::from(files.into_iter().collect::<Vec<FileApi>>()))
        }
        Err(SearchFileError::DbError) => SearchFileResponse::GenericError(BasicMessage::new(
            "Failed to search files. Check server logs for details",
        )),
        Err(SearchFileError::TagError) => SearchFileResponse::GenericError(BasicMessage::new(
            "Failed to retrieve file tags. Check server logs for details",
        )),
    }
}

#[get("/<id>")]
pub fn download_file(id: u32, auth: HeaderAuth) -> DownloadFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return DownloadFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DownloadFileResponse::Unauthorized("Bad Credentials".to_string())
    }
    match file_service::get_file_contents(id) {
        Ok(f) => DownloadFileResponse::Success(f),
        Err(GetFileError::NotFound) => DownloadFileResponse::FileNotFound(BasicMessage::new(
            "The file with the passed id could not be found.",
        )),
        Err(GetFileError::DbFailure) => DownloadFileResponse::FileDbError(BasicMessage::new(
            "Failed to retrieve the file info from the database. Check the server logs for details",
        )),
        Err(_) => panic!("Download file: We should never get here"),
    }
}

#[delete("/<id>")]
pub fn delete_file(id: u32, auth: HeaderAuth) -> DeleteFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return DeleteFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DeleteFileResponse::Unauthorized("Bad Credentials".to_string())
    };
    match file_service::delete_file(id) {
        Ok(()) => DeleteFileResponse::Deleted(()),
        Err(DeleteFileError::NotFound) => DeleteFileResponse::NotFound(BasicMessage::new(
            "The file with the passed id could not be found.",
        )),
        Err(DeleteFileError::DbError) => DeleteFileResponse::Failure(BasicMessage::new(
            "Failed to remove file reference from database.",
        )),
        Err(DeleteFileError::FileSystemError) => DeleteFileResponse::Failure(BasicMessage::new(
            "Failed to remove file from the file system.",
        )),
    }
}

#[put("/", data = "<data>")]
pub fn update_file(data: Json<FileApi>, auth: HeaderAuth) -> UpdateFileResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return UpdateFileResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return UpdateFileResponse::Unauthorized("Bad Credentials".to_string())
    };

    match file_service::update_file(data.into_inner()) {
        Ok(f) => UpdateFileResponse::Success(Json::from(f)),
        Err(UpdateFileError::NotFound) => UpdateFileResponse::NotFound(BasicMessage::new(
            "The file with the passed id could not be found.",
        )),
        Err(UpdateFileError::FolderNotFound) => UpdateFileResponse::NotFound(BasicMessage::new(
            "The folder with the passed id could not be found.",
        )),
        Err(UpdateFileError::FileAlreadyExists) => UpdateFileResponse::BadRequest(
            BasicMessage::new("A file with the same name already exists in the specified folder"),
        ),
        Err(UpdateFileError::FolderAlreadyExistsWithSameName) => UpdateFileResponse::BadRequest(
            BasicMessage::new("A folder with that name already exists."),
        ),
        Err(_) => UpdateFileResponse::GenericError(BasicMessage::new(
            "Failed to update the file. Check the server logs for details",
        )),
    }
}

#[get("/preview/<id>")]
pub fn get_file_preview(id: u32, auth: HeaderAuth) -> GetPreviewResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return GetPreviewResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetPreviewResponse::Unauthorized("Bad Credentials".to_string())
    };
    return match file_service::get_file_preview(id) {
        Ok(preview) => GetPreviewResponse::Success(preview),
        Err(GetPreviewError::NotFound) => GetPreviewResponse::NotFound(BasicMessage::new(
            "No preview for a file with that id could be found",
        )),
        Err(e) => GetPreviewResponse::GenericError(BasicMessage::new(
            format!("Failed to get file preview. Exception is {:?}", e).as_str(),
        )),
    };
}
