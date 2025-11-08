use rocket::http::Header;
use rocket::http::Status;
use rocket::local::blocking::Client;

use crate::rocket;
use crate::test::*;

fn client() -> Client {
    Client::tracked(rocket()).unwrap()
}

fn set_password() {
    init_db_folder();
    let client = client();
    let uri = uri!("/api/password");
    client
        .post(uri)
        .body(r#"{"username":"username","password":"password"}"#)
        .dispatch();
}

#[test]
fn version() {
    init_db_folder();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let res = client.get(uri!("/api/version")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.into_string().unwrap(), r#"{"version":"3.0.3"}"#);
    cleanup();
}

#[test]
fn set_password_missing_fields() {
    init_db_folder();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let uri = uri!("/api/password");
    let res = client.post(uri).dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    cleanup();
}

#[test]
fn set_password_works() {
    init_db_folder();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let uri = uri!("/api/password");
    let res = client
        .post(uri)
        .body(r#"{"username":"user","password":"password"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Created);
    cleanup();
}

#[test]
fn set_password_if_pass_exists() {
    set_password();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let uri = uri!("/api/password");
    let res = client
        .post(uri)
        .body(r#"{"username":"user","password":"password"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    cleanup();
}

#[test]
fn ping_without_credentials() {
    init_db_folder();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let uri = uri!("/api/ping");
    let res = client.get(uri).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn ping_with_valid_credentials() {
    set_password();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let uri = uri!("/api/ping");
    let res = client
        .get(uri)
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::NoContent);
    cleanup();
}

#[test]
fn ping_with_invalid_credentials() {
    set_password();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let uri = uri!("/api/ping");
    // wrong password
    let res = client
        .get(uri)
        .header(Header::new("Authorization", "Basic dXNlcm5hbWU6d3Jvbmc="))
        .dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}
