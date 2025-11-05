#[macro_use]
extern crate rocket;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs, time::Instant};

use rocket::{Build, Rocket};

use db_migrations::generate_all_file_types_and_sizes;
use handler::{api_handler::*, file_handler::*, folder_handler::*, tag_handler::*};

use crate::handler::api_handler::update_password;
use crate::previews::generate_preview;
use crate::queue::file_preview_consumer;
use crate::repository::initialize_db;

mod config;
mod db_migrations;
mod guard;
mod handler;
mod model;
mod previews;
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
    format!("./.{thread_name}_temp")
}

#[cfg(not(test))]
fn init_log() -> Result<(), fern::InitError> {
    // cargo fix keeps removing this if it's outside the function
    use std::time::SystemTime;
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

#[launch]
pub fn rocket() -> Rocket<Build> {
    #[cfg(not(test))]
    init_log().unwrap();
    initialize_db().unwrap();
    generate_all_file_types_and_sizes();
    fs::remove_dir_all(Path::new(temp_dir().as_str())).unwrap_or(());
    fs::create_dir(Path::new(temp_dir().as_str())).unwrap();
    // keep track of when the last request was made. This will let us wait for the server to be free before processing file previews
    let last_request_time: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
    file_preview_consumer(&last_request_time, generate_preview);
    // ik this isn't the right place for this, but it's a single line to prevent us from losing the directory
    // rocket needs this even during tests because it's configured in rocket.toml, and I can't change that value per test
    fs::write("./.file_server_temp/.gitkeep", "").unwrap();
    rocket::build()
        .mount(
            "/api",
            routes![api_version, set_password, update_password, get_disk_info],
        )
        .mount(
            "/files",
            routes![
                upload_file,
                get_file,
                delete_file,
                download_file,
                update_file,
                search_files,
                get_file_preview,
                regenerate_previews
            ],
        )
        .mount(
            "/folders",
            routes![
                get_folder,
                download_folder,
                create_folder,
                update_folder,
                delete_folder,
                get_child_file_previews
            ],
        )
        .mount(
            "/tags",
            routes![get_tag, create_tag, update_tag, delete_tag],
        )
        .mount("/previews", routes![previews::handler::get_folder_previews])
        .manage(last_request_time)
}

#[cfg(test)]
mod test;
