use rocket::serde::json::Json;

use crate::guard::Auth;
use crate::model::error::tag_errors::{GetTagError, UpdateTagError};
use crate::model::guard::auth::ValidateResult;
use crate::model::response::tag_responses::{
    CreateTagResponse, DeleteTagResponse, GetTagResponse, UpdateTagResponse,
};
use crate::model::response::{BasicMessage, TagApi};
use crate::service::tag_service;

#[get("/<id>")]
pub fn get_tag(id: u32, auth: Auth) -> GetTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return GetTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return GetTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    match tag_service::get_tag(id) {
        Ok(tag) => GetTagResponse::Success(Json::from(tag)),
        Err(e) if e == GetTagError::TagNotFound => GetTagResponse::TagNotFound(BasicMessage::new(
            "The tag with the passed id could not be found.",
        )),
        Err(_) => GetTagResponse::TagDbError(BasicMessage::new(
            "Failed to pull tag info from database. Check server logs for details",
        )),
    }
}

#[post("/", data = "<tag>")]
pub fn create_tag(tag: Json<TagApi>, auth: Auth) -> CreateTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return CreateTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return CreateTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    match tag_service::create_tag(tag.title.clone()) {
        Ok(tag) => CreateTagResponse::Success(Json::from(tag)),
        Err(_) => CreateTagResponse::TagDbError(BasicMessage::new(
            "Failed to create tag info in database. Check server logs for details",
        )),
    }
}

#[put("/", data = "<tag>")]
pub fn update_tag(tag: Json<TagApi>, auth: Auth) -> UpdateTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return UpdateTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return UpdateTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    match tag_service::update_tag(tag.into_inner()) {
        Ok(tag) => UpdateTagResponse::Success(Json::from(tag)),
        Err(e) if e == UpdateTagError::TagNotFound => {
            UpdateTagResponse::TagNotFound(BasicMessage::new("No tag with that id was found."))
        }
        Err(e) if e == UpdateTagError::NewNameAlreadyExists => UpdateTagResponse::TagAlreadyExists(
            BasicMessage::new("A tag with that name already exists."),
        ),
        Err(_) => UpdateTagResponse::TagDbError(BasicMessage::new(
            "Failed to update tag in database. Check server logs for details",
        )),
    }
}

#[delete("/<id>")]
pub fn delete_tag(id: u32, auth: Auth) -> DeleteTagResponse {
    match auth.validate() {
        ValidateResult::Ok => {/* no op */},
        ValidateResult::NoPasswordSet => return DeleteTagResponse::Unauthorized("No password has been set. you can set a username and password by making a POST to `/api/password`".to_string()),
        ValidateResult::Invalid => return DeleteTagResponse::Unauthorized("Bad Credentials".to_string())
    };
    match tag_service::delete_tag(id) {
        Ok(()) => DeleteTagResponse::Success(()),
        Err(_) => DeleteTagResponse::TagDbError(BasicMessage::new(
            "Failed to delete tag from database. Check server logs for details.",
        )),
    }
}
