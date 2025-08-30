use rocket::http::Status;
use rocket::local::blocking::Client;

use crate::rocket;
use crate::test::*;

fn client() -> Client {
    Client::tracked(rocket()).unwrap()
}

fn set_password() {
    refresh_db();
    let client = client();
    let uri = uri!("/api/password");
    client
        .post(uri)
        .body(r#"{"username":"username","password":"password"}"#)
        .dispatch();
}

#[test]
fn version() {
    refresh_db();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let res = client.get(uri!("/api/version")).dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.into_string().unwrap(), r#"{"version":"3.0.2"}"#);
    cleanup();
}

#[test]
fn set_password_missing_fields() {
    refresh_db();
    let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
    let uri = uri!("/api/password");
    let res = client.post(uri).dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    cleanup();
}

#[test]
fn set_password_works() {
    refresh_db();
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
