#[macro_use]
extern crate rocket;

use rocket::{Build, Rocket};

use crate::repository::initialize_db;
use handler::{
    api_handler::{api_version, set_password},
    file_handler::{delete_file, download_file, get_file, upload_file},
    folder_handler::{create_folder, delete_folder, get_folder, update_folder},
};

mod guard;
mod handler;
mod model;
mod repository;
mod service;

#[launch]
fn rocket() -> Rocket<Build> {
    initialize_db().unwrap();
    rocket::build()
        .mount("/api", routes![api_version, set_password])
        .mount(
            "/files",
            routes![upload_file, get_file, delete_file, download_file],
        )
        .mount(
            "/folders",
            routes![get_folder, create_folder, update_folder, delete_folder],
        )
}
