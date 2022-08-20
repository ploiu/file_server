extern crate core;
#[macro_use]
extern crate rocket;

use rocket::{Build, Rocket};

use handler::{
    api::{api_version, set_password},
    file::upload_file,
};

use crate::db::initialize_db;

mod db;
mod facade;
mod guard;
mod handler;
mod model;
mod service;

#[launch]
fn rocket() -> Rocket<Build> {
    initialize_db().unwrap();
    rocket::build()
        .mount("/api", routes![api_version, set_password])
        .mount("/file", routes![upload_file])
}
