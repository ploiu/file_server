use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use rocket::{State, response::stream::EventStream};

use crate::{
    guard::HeaderAuth,
    model::{guard::auth::ValidateResult, response::folder_responses::GetMultiPreviewResponse},
    previews::models::GetFolderPreviewsError,
    util::update_last_request_time,
};

#[get("/folder/<id>")]
pub fn get_folder_previews(
    id: u32,
    auth: HeaderAuth,
    last_request_time: &State<Arc<Mutex<Instant>>>,
) -> Result<EventStream![], GetFolderPreviewsError> {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return Err(GetFolderPreviewsError::Unauthorized("No password has been set. You can set a username and password by making a POST to `/api/password`".to_string())),
        ValidateResult::Invalid => return Err(GetFolderPreviewsError::Unauthorized("Bad Credentials".to_string()))
    };
    update_last_request_time(last_request_time);
    Ok(EventStream! {})
}
