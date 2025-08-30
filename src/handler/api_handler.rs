use std::sync::{Arc, Mutex};
use std::time::Instant;

use rocket::State;
use rocket::serde::{Serialize, json::Json};

use crate::guard::HeaderAuth;
use crate::model::error::metadata_errors::CreatePasswordError;
use crate::model::guard::auth::ValidateResult;
use crate::model::request::{BodyAuth, UpdateAuth};
use crate::model::response::BasicMessage;
use crate::model::response::api_responses::{
    GetDiskInfoResponse, SetPassWordResponse, UpdatePasswordResponse,
};
use crate::service::api_service::{self, DiskInfoError};
use crate::util::update_last_request_time;

static API_VERSION_NUMBER: &str = "3.0.2";

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiVersion {
    version: &'static str,
}

impl ApiVersion {
    fn new() -> ApiVersion {
        ApiVersion {
            version: API_VERSION_NUMBER,
        }
    }
}

#[get("/version")]
pub fn api_version() -> Json<ApiVersion> {
    Json(ApiVersion::new())
}

#[post("/password", data = "<auth>")]
pub fn set_password(auth: Json<BodyAuth>) -> SetPassWordResponse {
    let result = api_service::create_auth(auth.into_inner());
    match result {
        Ok(_) => SetPassWordResponse::Created(()),
        Err(CreatePasswordError::AlreadyExists) => SetPassWordResponse::AlreadyExists(
            BasicMessage::new("password cannot be set, as it already has been set"),
        ),
        Err(_) => SetPassWordResponse::Failure(BasicMessage::new(
            "Failed to set password due to unknown error",
        )),
    }
}

#[put("/password", data = "<auth>")]
pub fn update_password(auth: Json<UpdateAuth>) -> UpdatePasswordResponse {
    match api_service::update_auth(auth.into_inner()) {
        Ok(_) => UpdatePasswordResponse::Success(()),
        Err(_) => UpdatePasswordResponse::Unauthorized(()),
    }
}

#[get("/disk")]
pub fn get_disk_info(
    auth: HeaderAuth,
    last_request_time: &State<Arc<Mutex<Instant>>>,
) -> GetDiskInfoResponse {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return GetDiskInfoResponse::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetDiskInfoResponse::Unauthorized("Bad Credentials".to_string())
    };
    update_last_request_time(last_request_time);
    match api_service::get_disk_info() {
        Ok(info) => GetDiskInfoResponse::Success(Json::from(info)),
        Err(DiskInfoError::Generic) => {
            GetDiskInfoResponse::GenericError(BasicMessage::new("Failed to retrieve disk info"))
        }
        Err(DiskInfoError::Windows) => GetDiskInfoResponse::Windows(BasicMessage::new(
            "Disk info support isn't available for server running on windows",
        )),
    }
}
