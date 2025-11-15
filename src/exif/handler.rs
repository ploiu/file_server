use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use rocket::{State, http::Status};

use crate::{
    guard::HeaderAuth, model::guard::auth::ValidateResult, util::update_last_request_time,
};

#[get("/regen")]
pub fn regenerate_exif(auth: HeaderAuth, last_request_time: &State<Arc<Mutex<Instant>>>) -> Status {
    match auth.validate() {
        ValidateResult::Ok => { /*no op*/ }
        ValidateResult::NoPasswordSet => return Status::Unauthorized,
        ValidateResult::Invalid => return Status::Unauthorized,
    };
    update_last_request_time(last_request_time);

    std::thread::spawn(|| {
        super::service::mass_exif_process();
    });

    Status::Accepted
}
