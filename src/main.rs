#[macro_use]
extern crate rocket;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs, time::Instant};

use rocket::{Build, Rocket};
use service::preview_service::generate_preview;

#[cfg(not(test))]
use simple_logger::SimpleLogger;

use handler::{
    api_handler::{api_version, set_password},
    file_handler::{
        delete_file, download_file, get_file, get_file_preview, search_files, update_file,
        upload_file,
    },
    folder_handler::{create_folder, delete_folder, get_folder, update_folder},
    tag_handler::{create_tag, delete_tag, get_tag, update_tag},
};

use crate::handler::api_handler::update_password;
use crate::queue::file_preview_consumer;
use crate::repository::initialize_db;

mod config;
mod guard;
mod handler;
mod model;
mod queue;
mod repository;
mod service;
mod util;

#[cfg(not(test))]
fn temp_dir() -> String {
    "./.file_server_temp".to_string()
}

#[cfg(test)]
fn temp_dir() -> String {
    let thread_name = test::current_thread_name();
    format!("./.{}_temp", thread_name)
}

/// the way tests run for rocket mean logging would be initialized multiple times, which causes errors
fn init_log() {
    #[cfg(not(test))]
    SimpleLogger::new().env().init().unwrap();
}

#[launch]
pub fn rocket() -> Rocket<Build> {
    init_log();
    initialize_db().unwrap();
    fs::remove_dir_all(Path::new(temp_dir().as_str())).unwrap_or(());
    fs::create_dir(Path::new(temp_dir().as_str())).unwrap();
    // keep track of when the last request was made. This will let us wait for the server to be free before processing file previews
    let last_request_time: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
    file_preview_consumer(&last_request_time, generate_preview);
    // ik this isn't the right place for this, but it's a single line to prevent us from losing the directory
    // rocket needs this even during tests because it's configured in rocket.toml, and I can't change that value per test
    fs::write("./.file_server_temp/.gitkeep", "").unwrap();
    rocket::build()
        .mount("/api", routes![api_version, set_password, update_password])
        .mount(
            "/files",
            routes![
                upload_file,
                get_file,
                delete_file,
                download_file,
                update_file,
                search_files,
                get_file_preview
            ],
        )
        .mount(
            "/folders",
            routes![get_folder, create_folder, update_folder, delete_folder],
        )
        .mount(
            "/tags",
            routes![get_tag, create_tag, update_tag, delete_tag],
        )
        .manage(last_request_time)
}

#[cfg(test)]
mod test;
