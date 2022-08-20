use rocket::http::Status;
use rocket::serde::{json::Json, Serialize};

use crate::model::request::NewAuth;
use crate::model::response::BasicResponse;
use crate::service::api_service;

static API_VERSION_NUMBER: f64 = 0.1;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiVersion {
    version: f64,
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
pub fn set_password<'a>(auth: Json<NewAuth>) -> (Status, BasicResponse<'a>) {
    let result = api_service::create_password(auth.into_inner());
    return match result {
        Ok(_) => BasicResponse::text(Status::NoContent, ""),
        Err(reason) => BasicResponse::json(Status::InternalServerError, reason),
    };
}
