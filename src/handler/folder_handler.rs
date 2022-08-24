use crate::guard::{Auth, ValidateResult};
use crate::model::response::folder_responses::GetFolderResponse;
use crate::model::response::BasicMessage;
use crate::service::folder_service;
use crate::service::folder_service::GetFolderError;
use rocket::serde::json::Json;

#[get("/<id>")]
pub fn get_folder(id: u64, auth: Auth) -> GetFolderResponse {
    match auth.validate() {
        ValidateResult::Ok => {/*no op*/}
        ValidateResult::NoPasswordSet => return GetFolderResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetFolderResponse::Unauthorized("Bad Credentials".to_string())
    };
    return match folder_service::get_folder(id) {
        Ok(folder) => GetFolderResponse::Success(Json::from(folder)),
        Err(message) if message == GetFolderError::NotFound => GetFolderResponse::FileNotFound(
            BasicMessage::new("The folder with the passed id could not be found."),
        ),
        // TODO maybe distinguish between not found on disk and not able to pull in DB?
        Err(_) => GetFolderResponse::FileDbError(BasicMessage::new(
            "Failed to pull folder info from database. Check server logs for details",
        )),
    };
}
