use rocket::http::Status;
use rocket::local::blocking::Client;

use crate::rocket;
use crate::test::{cleanup, init_db_folder, remove_files};

fn client() -> Client {
    Client::tracked(rocket()).unwrap()
}

#[test]
fn should_require_auth() {
    // ensure db initialized but no password set
    init_db_folder();
    remove_files();
    let client = client();
    let res = client.get("/exif/regen").dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn should_reject_invalid_auth() {
    init_db_folder();
    remove_files();
    let client = client();
    // provide an invalid basic auth header (wrong credentials)
    let res = client
        .get("/exif/regen")
        .header(rocket::http::Header::new(
            "Authorization",
            "Basic d3Jvbmc6Y3JlZGVudHM=",
        ))
        .dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn should_reject_if_no_password_set() {
    // initialize without setting a password
    init_db_folder();
    remove_files();
    let client = client();
    // explicitly no Authorization header should return Unauthorized when no password is set
    let res = client.get("/exif/regen").dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}
