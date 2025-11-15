use std::sync::{Arc, Mutex};
use std::time::Instant;

use rocket::State;
use rocket::serde::json::Json;

use crate::guard::HeaderAuth;
use crate::model::error::tag_errors::{GetTagError, UpdateTagError};
use crate::model::guard::auth::ValidateResult;
use crate::model::response::tag_responses::{
    CreateTagResponse, DeleteTagResponse, GetTagResponse, UpdateTagResponse,
};
use crate::model::response::{BasicMessage, TagApi};
use crate::tags::service;
use crate::util::update_last_request_time;

#[get("/<id>")]
pub fn get_tag(
    id: u32,
    auth: HeaderAuth,
    last_request_time: &State<Arc<Mutex<Instant>>>,
) -> GetTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return GetTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    update_last_request_time(last_request_time);
    match service::get_tag(id) {
        Ok(tag) => GetTagResponse::Success(Json::from(tag)),
        Err(GetTagError::TagNotFound) => GetTagResponse::TagNotFound(BasicMessage::new(
            "The tag with the passed id could not be found.",
        )),
        Err(_) => GetTagResponse::TagDbError(BasicMessage::new(
            "Failed to pull tag info from database. Check server logs for details",
        )),
    }
}

#[post("/", data = "<tag>")]
pub fn create_tag(
    tag: Json<TagApi>,
    auth: HeaderAuth,
    last_request_time: &State<Arc<Mutex<Instant>>>,
) -> CreateTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return CreateTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return CreateTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    update_last_request_time(last_request_time);
    match service::create_tag(tag.title.clone()) {
        Ok(tag) => CreateTagResponse::Success(Json::from(tag)),
        Err(_) => CreateTagResponse::TagDbError(BasicMessage::new(
            "Failed to create tag info in database. Check server logs for details",
        )),
    }
}

#[put("/", data = "<tag>")]
pub fn update_tag(
    tag: Json<TagApi>,
    auth: HeaderAuth,
    last_request_time: &State<Arc<Mutex<Instant>>>,
) -> UpdateTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return UpdateTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return UpdateTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    update_last_request_time(last_request_time);
    match service::update_tag(tag.into_inner()) {
        Ok(tag) => UpdateTagResponse::Success(Json::from(tag)),
        Err(UpdateTagError::TagNotFound) => {
            UpdateTagResponse::TagNotFound(BasicMessage::new("No tag with that id was found."))
        }
        Err(UpdateTagError::NewNameAlreadyExists) => UpdateTagResponse::TagAlreadyExists(
            BasicMessage::new("A tag with that name already exists."),
        ),
        Err(_) => UpdateTagResponse::TagDbError(BasicMessage::new(
            "Failed to update tag in database. Check server logs for details",
        )),
    }
}

#[delete("/<id>")]
pub fn delete_tag(
    id: u32,
    auth: HeaderAuth,
    last_request_time: &State<Arc<Mutex<Instant>>>,
) -> DeleteTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return DeleteTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DeleteTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    update_last_request_time(last_request_time);
    match service::delete_tag(id) {
        Ok(()) => DeleteTagResponse::Success(()),
        Err(_) => DeleteTagResponse::TagDbError(BasicMessage::new(
            "Failed to delete tag from database. Check server logs for details.",
        )),
    }
}
