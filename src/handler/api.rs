use rocket::serde::{json::Json, Serialize};

static API_VERSION_NUMBER: f64 = 0.1;
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiVersion {
    version: f64,
}

impl ApiVersion {
    fn new() -> ApiVersion {
        ApiVersion { version: API_VERSION_NUMBER }
    }
}

#[get("/version")]
pub fn api_version() -> Json<ApiVersion> {
    Json(ApiVersion::new())
}