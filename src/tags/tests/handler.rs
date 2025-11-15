use rocket::http::{Header, Status};
use rocket::local::blocking::Client;

use crate::model::response::TagApi;
use crate::repository::initialize_db;
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
fn get_tag_without_creds() {
    initialize_db().unwrap();
    let client = client();
    let res = client.get(uri!("/tags/1")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn get_tag_success() {
    set_password();
    create_tag_db_entry("test_tag");
    let client = client();
    let auth = Header::new("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    let res = client.get(uri!("/tags/1")).header(auth).dispatch();
    assert_eq!(res.status(), Status::Ok);
    cleanup();
}

#[test]
fn get_tag_not_found() {
    set_password();
    let client = client();
    let auth = Header::new("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    let res = client.get(uri!("/tags/999")).header(auth).dispatch();
    assert_eq!(res.status(), Status::NotFound);
    cleanup();
}

#[test]
fn create_tag_without_creds() {
    initialize_db().unwrap();
    let client = client();
    let res = client
        .post(uri!("/tags"))
        .body(r#"{"title":"new_tag"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn create_tag_success() {
    set_password();
    let client = client();
    let auth = Header::new("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    let res = client
        .post(uri!("/tags"))
        .header(auth)
        .body(r#"{"title":"new_tag"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Created);
    cleanup();
}

#[test]
fn update_tag_without_creds() {
    initialize_db().unwrap();
    let client = client();
    let res = client
        .put(uri!("/tags"))
        .body(r#"{"id":1,"title":"updated_tag"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn update_tag_success() {
    set_password();
    create_tag_db_entry("original_tag");
    let client = client();
    let auth = Header::new("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    let res = client
        .put(uri!("/tags"))
        .header(auth)
        .body(r#"{"id":1,"title":"updated_tag"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    cleanup();
}

#[test]
fn update_tag_not_found() {
    set_password();
    let client = client();
    let auth = Header::new("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    let res = client
        .put(uri!("/tags"))
        .header(auth)
        .body(r#"{"id":999,"title":"updated_tag"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    cleanup();
}

#[test]
fn update_tag_already_exists() {
    set_password();
    create_tag_db_entry("tag1");
    create_tag_db_entry("tag2");
    let client = client();
    let auth = Header::new("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    let res = client
        .put(uri!("/tags"))
        .header(auth)
        .body(r#"{"id":2,"title":"tag1"}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    cleanup();
}

#[test]
fn delete_tag_without_creds() {
    initialize_db().unwrap();
    let client = client();
    let res = client.delete(uri!("/tags/1")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn delete_tag_success() {
    set_password();
    create_tag_db_entry("test_tag");
    let client = client();
    let auth = Header::new("Authorization", "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
    let res = client.delete(uri!("/tags/1")).header(auth).dispatch();
    assert_eq!(res.status(), Status::NoContent);
    cleanup();
}
