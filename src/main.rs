extern crate core;
#[macro_use]
extern crate rocket;

use rocket::{Build, Rocket};

use handler::{
    api_handler::{api_version, set_password},
    file_handler::{delete_file, get_file, upload_file},
    folder_handler::{create_folder, get_folder, update_folder},
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
        .mount("/files", routes![upload_file, get_file, delete_file])
        .mount(
            "/folders",
            routes![get_folder, create_folder, update_folder],
        )
}
