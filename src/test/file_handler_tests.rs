use std::{fs, vec};

use rocket::http::{Header, Status};
use rocket::local::blocking::Client;
use rocket::serde::json::serde_json as serde;

use crate::model::api::FileApi;
use crate::model::file_types::FileTypes;
use crate::model::response::BasicMessage;
use crate::repository::initialize_db;
use crate::rocket;
use crate::service::file_service::file_dir;
use crate::test;
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
fn upload_file_without_creds() {
    initialize_db().unwrap();
    remove_files();
    let client = client();
    let res = client.post(uri!("/files")).dispatch();
    // without a password set
    assert_eq!(res.status(), Status::Unauthorized);
    // now with a password set
    set_password();
    let res = client.post(uri!("/files")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn upload_file_already_exists_no_query_param_root() {
    set_password();
    remove_files();
    test::create_file_db_entry("test.txt", None);
    create_file_disk("test.txt", "test");
    let client = client();
    let body = "--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
aGk=\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"extension\"\r\n\
\r\n\
txt\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"folderId\"\r\n\
\r\n\
0\r\n\
--BOUNDARY--";
    let res = client
        .post("/files")
        .header(Header::new("Authorization", AUTH))
        .header(Header::new(
            "Content-Type",
            "multipart/form-data; boundary=BOUNDARY",
        ))
        .body(body)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let res_body: BasicMessage = res.into_json().unwrap();
    assert_eq!("That file already exists", res_body.message);
    // ensure we didn't overwrite the file on the disk
    let disk_file = fs::read_to_string(format!("{}/{}", file_dir(), "test.txt")).unwrap();
    assert_eq!("test", disk_file);
    cleanup();
}

#[test]
fn upload_file_already_exists_no_query_param_sub_folder() {
    set_password();
    remove_files();
    test::create_folder_db_entry("test", None);
    create_folder_disk("test");
    test::create_file_db_entry("test.txt", Some(1));
    create_file_disk("test/test.txt", "test");
    let client = client();
    let body = "--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
aGk=\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"extension\"\r\n\
\r\n\
txt\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"folderId\"\r\n\
\r\n\
1\r\n\
--BOUNDARY--";
    let res = client
        .post("/files")
        .header(Header::new("Authorization", AUTH))
        .header(Header::new(
            "Content-Type",
            "multipart/form-data; boundary=BOUNDARY",
        ))
        .body(body)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let res_body: BasicMessage = res.into_json().unwrap();
    assert_eq!("That file already exists", res_body.message);
    // ensure we didn't overwrite the file on the disk
    let disk_file = fs::read_to_string(format!("{}/test/{}", file_dir(), "test.txt")).unwrap();
    assert_eq!("test", disk_file);
    cleanup();
}

#[test]
fn upload_file_already_exists_with_query_param_root() {
    set_password();
    remove_files();
    test::create_file_db_entry("test.txt", None);
    create_file_disk("test.txt", "test");
    let client = client();
    let body = "--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
aGk=\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"extension\"\r\n\
\r\n\
txt\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"folderId\"\r\n\
\r\n\
0\r\n\
--BOUNDARY--";
    let res = client
        .post("/files?force")
        .header(Header::new("Authorization", AUTH))
        .header(Header::new(
            "Content-Type",
            "multipart/form-data; boundary=BOUNDARY",
        ))
        .body(body)
        .dispatch();
    assert_eq!(res.status(), Status::Created);
    // ensure the file was overwritten
    let disk_file = fs::read_to_string(format!("{}/{}", file_dir(), "test.txt")).unwrap();
    assert_eq!("aGk=", disk_file);
    cleanup();
}

#[test]
fn upload_file_already_exists_with_query_param_sub_folder() {
    set_password();
    remove_files();
    test::create_folder_db_entry("test", None);
    create_folder_disk("test");
    test::create_file_db_entry("test.txt", Some(1));
    create_file_disk("test/test.txt", "test");
    let client = client();
    let body = "--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
aGk=\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"extension\"\r\n\
\r\n\
txt\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"folderId\"\r\n\
\r\n\
1\r\n\
--BOUNDARY--";
    let res = client
        .post("/files?force")
        .header(Header::new("Authorization", AUTH))
        .header(Header::new(
            "Content-Type",
            "multipart/form-data; boundary=BOUNDARY",
        ))
        .body(body)
        .dispatch();
    assert_eq!(res.status(), Status::Created);
    // ensure the file was overwritten
    let disk_file = fs::read_to_string(format!("{}/test/{}", file_dir(), "test.txt")).unwrap();
    assert_eq!("aGk=", disk_file);
    cleanup();
}

#[test]
fn upload_file() {
    set_password();
    remove_files();
    let client = client();
    let body = "--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
aGk=\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"extension\"\r\n\
\r\n\
txt\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"folderId\"\r\n\
\r\n\
0\r\n\
--BOUNDARY--";

    let res = client
        .post(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        .header(Header::new(
            "Content-Type",
            "multipart/form-data; boundary=BOUNDARY",
        ))
        .body(body)
        .dispatch();
    assert_eq!(res.status(), Status::Created);
    let res_body: FileApi = res.into_json().unwrap();
    assert_eq!(res_body.id, 1);
    assert_eq!(res_body.name, "test.txt".to_string());
    assert_eq!(res_body.folder_id, None);
    assert_eq!(res_body.tags, vec![]);
    assert_eq!(res_body.file_type, Some(FileTypes::Text));
    assert_eq!(res_body.size, Some(6));
    cleanup();
}

#[test]
fn upload_file_parent_not_found() {
    set_password();
    remove_files();
    let client = client();
    let body = "--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
aGk=\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"extension\"\r\n\
\r\n\
txt\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"folderId\"\r\n\
\r\n\
1\r\n\
--BOUNDARY--";

    let res = client
        .post(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        .header(Header::new(
            "Content-Type",
            "multipart/form-data; boundary=BOUNDARY",
        ))
        .body(body)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    cleanup();
}

#[test]
fn upload_file_without_extension() {
    set_password();
    remove_files();
    let client = client();
    let body = "--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"test\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
aGk=\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Disposition: form-data; name=\"folderId\"\r\n\
\r\n\
0\r\n\
--BOUNDARY--";
    let res = client
        .post(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        .header(Header::new(
            "Content-Type",
            "multipart/form-data; boundary=BOUNDARY",
        ))
        .body(body)
        .dispatch();
    let status = res.status();
    assert_eq!(status, Status::Created);
    let res_body: FileApi = res.into_json().unwrap();
    assert_eq!(res_body.id, 1);
    assert_eq!(res_body.name, "test".to_string());
    assert_eq!(res_body.folder_id, None);
    assert_eq!(res_body.tags, vec![]);
    assert_eq!(res_body.file_type, Some(FileTypes::Unknown));
    assert_eq!(res_body.size, Some(6));

    // make sure that the file comes back with the right name
    let res: FileApi = client
        .get(uri!("/files/metadata/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch()
        .into_json()
        .unwrap();
    assert_eq!(res.id, 1);
    assert_eq!(res.name, "test".to_string());
    assert_eq!(res.folder_id, None);
    assert_eq!(res.tags, vec![]);
    assert_eq!(res.file_type, Some(FileTypes::Unknown));
    assert_eq!(res.size, Some(6));
    cleanup();
}

#[test]
fn get_file_without_creds() {
    initialize_db().unwrap();
    remove_files();
    let client = client();
    let res = client.get(uri!("/files/metadata/1234")).dispatch();
    // without a password set
    assert_eq!(res.status(), Status::Unauthorized);
    // now with a password set
    set_password();
    let res = client.get(uri!("/files/metadata/1234")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn get_file_not_found() {
    set_password();
    remove_files();
    let client = client();
    let res = client
        .get(uri!("/files/metadata/1234"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    cleanup();
}

#[test]
fn get_file() {
    set_password();
    remove_files();
    create_file_db_entry("file_name.txt", None);
    let client = client();
    let res = client
        .get(uri!("/files/metadata/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    let status = res.status();
    let body: FileApi = res.into_json().unwrap();
    assert_eq!(status, Status::Ok);
    assert_eq!(body.name, String::from("file_name.txt"));
    assert_eq!(body.id, 1);
    cleanup();
}

#[test]
fn search_files_without_creds() {
    refresh_db();
    remove_files();
    let client = client();
    let res = client.get(uri!("/files/metadata?search=test")).dispatch();
    // without a password set
    assert_eq!(res.status(), Status::Unauthorized);
    // now with a password set
    set_password();
    let res = client.get(uri!("/files/metadata?search=test")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn search_files_bad_search_query() {
    set_password();
    remove_files();
    let client = client();
    let res = client
        .get("/files/metadata?search")
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: BasicMessage = res.into_json().unwrap();
    assert_eq!(
        body.message,
        String::from("Search string, attributes, or tags are required.")
    );
    cleanup();
}

#[test]
fn search_files() {
    set_password();
    remove_files();
    // need to add to the database
    test::create_file_db_entry("should_return.txt", None);
    test::create_file_db_entry("should_not_return.txt", None);
    let client = client();
    let res = client
        .get("/files/metadata?search=should_return")
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: Vec<FileApi> = res.into_json().unwrap();
    assert_eq!(body.len(), 1);
    let file = &body[0];
    assert_eq!(file.id, 1);
    assert_eq!(file.name, String::from("should_return.txt"));
    cleanup();
}

#[test]
fn download_file_without_creds() {
    refresh_db();
    remove_files();
    let client = client();
    let res = client.get(uri!("/files/1")).dispatch();
    // without a password set
    assert_eq!(res.status(), Status::Unauthorized);
    // now with a password set
    set_password();
    let res = client.get(uri!("/files/1")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn download_file_not_found() {
    set_password();
    remove_files();
    let client = client();
    let res = client
        .get(uri!("/files/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    let body: BasicMessage = res.into_json().unwrap();
    assert_eq!(
        body.message,
        String::from("The file with the passed id could not be found.")
    );
    cleanup();
}

#[test]
fn download_file() {
    set_password();
    remove_files();
    test::create_file_db_entry("test.txt", None);
    create_file_disk("test.txt", "hello");
    let client = client();
    let res = client
        .get(uri!("/files/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    // TODO res content type test
    let body: String = res.into_string().unwrap();
    assert_eq!(body, String::from("hello"));
    cleanup();
}

#[test]
fn delete_file_without_creds() {
    refresh_db();
    remove_files();
    let client = client();
    let res = client.delete(uri!("/files/1")).dispatch();
    // without a password set
    assert_eq!(res.status(), Status::Unauthorized);
    // now with a password set
    set_password();
    let res = client.delete(uri!("/files/1")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn delete_file_not_found() {
    set_password();
    remove_files();
    let client = client();
    let res = client
        .delete(uri!("/files/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    let body: BasicMessage = res.into_json().unwrap();
    assert_eq!(
        body.message,
        String::from("The file with the passed id could not be found.")
    );
    cleanup();
}

#[test]
fn delete_file() {
    refresh_db();
    set_password();
    test::create_file_db_entry("test.txt", None);
    create_file_disk("test.txt", "hi");
    let client = client();
    let res = client
        .delete(uri!("/files/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(res.status(), Status::NoContent);
    // make sure the file was removed from the disk and db
    if fs::read(format!("{}/{}", file_dir(), "test.txt")).is_ok() {
        fail()
    };
    let get_res = client
        .get(uri!("/files/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(get_res.status(), Status::NotFound);
    cleanup();
}

#[test]
fn update_file_without_creds() {
    refresh_db();
    remove_files();
    let client = client();
    let res = client.put(uri!("/files")).dispatch();
    // without a password set
    assert_eq!(res.status(), Status::Unauthorized);
    // now with a password set
    set_password();
    let res = client.put(uri!("/files")).dispatch();
    assert_eq!(res.status(), Status::Unauthorized);
    cleanup();
}

#[test]
fn update_file_file_not_found() {
    set_password();
    remove_files();
    let client = client();
    let res = client
        .put(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        //language=json
        .body(r#"{"id": 1, "name": "test","folderId": null, "tags":  []}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    let body: BasicMessage = res.into_json().unwrap();
    assert_eq!(
        body.message,
        String::from("The file with the passed id could not be found.")
    );
    cleanup();
}

#[test]
fn update_file_target_folder_not_found() {
    set_password();
    remove_files();
    test::create_file_db_entry("test", None);
    create_file_disk("test", "test");
    let client = client();
    let res = client
        .put(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        //language=json
        .body(r#"{"id": 1, "name": "test", "folderId": 1, "tags": []}"#)
        .dispatch();
    assert_eq!(res.status(), Status::NotFound);
    let body: BasicMessage = res.into_json().unwrap();
    assert_eq!(
        body.message,
        String::from("The folder with the passed id could not be found.")
    );
    cleanup();
}

#[test]
fn update_file_file_already_exists() {
    set_password();
    remove_files();
    test::create_file_db_entry("test.txt", None);
    test::create_file_db_entry("test2.txt", None);
    create_file_disk("test.txt", "test");
    create_file_disk("test2.txt", "test2");
    let client = client();
    let res = client
        .put(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        //language=json ; rename test.txt to test2.txt
        .body(r#"{"id": 1,"name": "test2.txt","parentId": null, "tags":  []}"#)
        .dispatch();
    assert_eq!(res.status(), Status::BadRequest);
    let body: BasicMessage = res.into_json().unwrap();
    assert_eq!(
        body.message,
        String::from("A file with the same name already exists in the specified folder")
    );
    cleanup();
}

#[test]
fn update_file_no_extension_removes_extension_and_file_type() {
    set_password();
    remove_files();
    create_file_db_entry("test.txt", None);
    create_file_disk("test.txt", "test");
    let client = client();
    let body = serde::to_string(&FileApi::new(1, Some(0), "test".to_string())).unwrap();
    let res = client
        .put(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        .header(Header::new("Content-Type", "application/json"))
        .body(body)
        .dispatch();
    let status = res.status();
    assert_eq!(status, Status::Ok);
    let res_body: FileApi = res.into_json().unwrap();
    assert_eq!(res_body.id, 1);
    assert_eq!(res_body.name, "test".to_string());
    assert_eq!(res_body.folder_id, None);
    assert_eq!(res_body.tags, vec![]);
    assert_eq!(res_body.size, Some(0));
    assert_eq!(res_body.file_type, Some(FileTypes::Unknown));
    cleanup();
}

#[test]
fn update_file() {
    set_password();
    remove_files();
    create_folder_db_entry("target_folder", None); // id 1
    create_file_db_entry("test.txt", None); // id 1
    create_file_db_entry("other.txt", Some(1)); // id 2
    create_file_disk("test.txt", "test"); // (1)
    create_folder_disk("target_folder"); // (1)
    create_file_disk("target_folder/other.txt", "other"); // (2)
    let client = client();
    let res = client
        .put(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        //language=json
        .body(r#"{"id": 1, "name": "new_name.txt", "folderId": 1, "tags":  []}"#)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: FileApi = res.into_json().unwrap();
    assert_eq!(body.id, 1);
    assert_eq!(body.name, String::from("new_name.txt"));
    cleanup();
}

#[test]
fn update_file_name_collides_with_folder() {
    set_password();
    remove_files();
    create_folder_db_entry("test", None); // id 1
    create_file_db_entry("file", None); // id 1
    let client = client();
    let req = serde::to_string(&FileApi::new(1, Some(0), "test".to_string())).unwrap();
    let res = client
        .put(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        .header(Header::new("Content-Type", "application/json"))
        .body(req)
        .dispatch();
    let status = res.status();
    let res_body: BasicMessage = res.into_json().unwrap();
    assert_eq!(status, Status::BadRequest);
    assert_eq!(res_body.message, "A folder with that name already exists.");
    cleanup();
}

#[test]
fn test_update_file_trailing_name_fix() {
    set_password();
    remove_files();
    create_file_db_entry("test_thing.txt", None);
    create_file_disk("test_thing.txt", "test_thing");
    create_folder_db_entry("inner", None);
    create_folder_disk("inner");
    create_file_db_entry("thing.txt", Some(1));
    create_file_disk("inner/thing.txt", "thing");
    let client = client();
    let req = serde::to_string(&FileApi::new(2, None, "thing.txt".to_string())).unwrap();
    let res = client
        .put(uri!("/files"))
        .header(Header::new("Authorization", AUTH))
        .header(Header::new("Content-Type", "application/json"))
        .body(req)
        .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let body: FileApi = res.into_json().unwrap();
    assert_eq!(body.id, 2);
    assert_eq!(body.name, String::from("thing.txt"));
    cleanup();
}
