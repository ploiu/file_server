use rocket::serde::{json::Json, Serialize};

use crate::model::error::metadata_errors::CreatePasswordError;
use crate::model::request::NewAuth;
use crate::model::response::api_responses::SetPassWordResponse;
use crate::model::response::BasicMessage;
use crate::service::api_service;

static API_VERSION_NUMBER: &str = "2.1.0";

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
pub fn set_password(auth: Json<NewAuth>) -> SetPassWordResponse {
    let result = api_service::create_password(auth.into_inner());
    return match result {
        Ok(_) => SetPassWordResponse::Created(()),
        Err(e) if e == CreatePasswordError::AlreadyExists => SetPassWordResponse::AlreadyExists(
            BasicMessage::new("password cannot be set, as it already has been set"),
        ),
        Err(_) => SetPassWordResponse::Failure(BasicMessage::new(
            "Failed to set password due to unknown error",
        )),
    };
}
