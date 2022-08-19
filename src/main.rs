#[macro_use]
extern crate rocket;
extern crate core;

use rocket::{Build, Rocket};

use crate::db::initialize_db;
use handler::{api::api_version, file::upload_file};

mod db;
mod facade;
mod guard;
mod handler;
mod model;

#[launch]
fn rocket() -> Rocket<Build> {
    initialize_db().unwrap();
    rocket::build()
        .mount("/api", routes![api_version])
        .mount("/file", routes![upload_file])
}
