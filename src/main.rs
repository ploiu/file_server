#[macro_use] extern crate rocket;

use rocket::{Build, Rocket};

use handler::{
    api::api_version,
    file::upload_file
};

mod handler;
mod model;

#[launch]
fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount("/api", routes![api_version])
        .mount("/file", routes![upload_file])
}