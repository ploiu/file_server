#[macro_use]
extern crate rocket;

use rocket::{Build, Rocket};

use handler::{
    api_handler::{api_version, set_password},
    file_handler::{delete_file, download_file, get_file, search_files, update_file, upload_file},
    folder_handler::{create_folder, delete_folder, get_folder, update_folder},
};

use crate::repository::initialize_db;

mod guard;
mod handler;
mod model;
mod repository;
mod service;
mod test;

#[launch]
fn rocket() -> Rocket<Build> {
    initialize_db().unwrap();
    rocket::build()
        .mount("/api", routes![api_version, set_password])
        .mount(
            "/files",
            routes![
                upload_file,
                get_file,
                delete_file,
                download_file,
                update_file,
                search_files
            ],
        )
        .mount(
            "/folders",
            routes![get_folder, create_folder, update_folder, delete_folder],
        )
}

///
/// Look at .run/test.run.xml for run arguments - since there's file system ops we need to run with 1 thread
///
#[cfg(test)]
mod api_tests {
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    use crate::test::refresh_db;

    use super::rocket;

    #[test]
    fn version() {
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
        let res = client.get(uri!("/api/version")).dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_string().unwrap(), r#"{"version":"1.0.0"}"#);
    }

    #[test]
    fn set_password_missing_fields() {
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
        let uri = uri!("/api/password");
        let res = client.post(uri).dispatch();
        assert_eq!(res.status(), Status::BadRequest);
    }

    #[test]
    fn set_password() {
        refresh_db();
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
        let uri = uri!("/api/password");
        let res = client
            .post(uri)
            .body(r#"{"username":"user","password":"password"}"#)
            .dispatch();
        assert_eq!(res.status(), Status::Created);
    }

    #[test]
    fn set_password_if_pass_exists() {
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
        let uri = uri!("/api/password");
        let res = client
            .post(uri)
            .body(r#"{"username":"user","password":"password"}"#)
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
    }
}

#[cfg(test)]
mod folder_tests {
    use std::path::Path;

    use rocket::http::{Header, Status};
    use rocket::local::blocking::Client;
    use rocket::serde::json::serde_json as serde;

    use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::BasicMessage;
    use crate::repository::initialize_db;
    use crate::service::file_service::FILE_DIR;
    use crate::test::{refresh_db, remove_files, AUTH};

    use super::rocket;

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
    fn get_root_folder() {
        set_password();
        remove_files();
        let client = client();
        let uri = uri!("/folders/null");
        let res = client
            .get(uri)
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        let expected = FolderResponse {
            id: 0,
            parent_id: None,
            path: String::from("root"),
            folders: Vec::new(),
            files: Vec::new(),
        };
        let status = res.status();
        let res_json: FolderResponse = res.into_json().unwrap();
        assert_eq!(status, Status::Ok);
        assert_eq!(res_json, expected);
    }

    #[test]
    fn get_non_existent_folder() {
        set_password();
        remove_files();
        let client = client();
        let uri = uri!("/folders/1234");
        let res = client
            .get(uri)
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        let expected =
            BasicMessage::new("The folder with the passed id could not be found.").into_inner();
        let status = res.status();
        let res_json: BasicMessage = res.into_json().unwrap();
        assert_eq!(status, Status::NotFound);
        assert_eq!(res_json, expected);
    }

    #[test]
    fn get_folder_without_creds() {
        initialize_db().unwrap();
        remove_files();
        let client = client();
        let res = client.get(uri!("/folders/1234")).dispatch();
        // without a password set
        assert_eq!(res.status(), Status::Unauthorized);
        // now with a password set
        set_password();
        let res = client.get(uri!("/folders/1234")).dispatch();
        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[test]
    fn create_folder_without_creds() {
        initialize_db().unwrap();
        remove_files();
        let client = client();
        let res = client.post(uri!("/folders")).dispatch();
        // without a password set
        assert_eq!(res.status(), Status::Unauthorized);
        // now with a password set
        set_password();
        let res = client.post(uri!("/folders")).dispatch();
        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[test]
    fn create_folder_non_existent() {
        set_password();
        remove_files();
        let client = client();
        let req_body = CreateFolderRequest {
            name: String::from("whatever"),
            parent_id: None,
        };
        let res = client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(serde::to_string(&req_body).unwrap())
            .dispatch();
        assert_eq!(res.status(), Status::Created);
    }

    #[test]
    fn create_folder_already_exists() {
        set_password();
        remove_files();
        let client = client();
        let req_body = CreateFolderRequest {
            name: String::from("whatever"),
            parent_id: None,
        };
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(serde::to_string(&req_body).unwrap())
            .dispatch();
        let res = client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(serde::to_string(&req_body).unwrap())
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(
            body,
            BasicMessage {
                message: String::from("That folder already exists.")
            }
        );
    }

    #[test]
    fn create_folder_parent_not_found() {
        set_password();
        remove_files();
        let client = client();
        let req_body = CreateFolderRequest {
            name: String::from("whatever"),
            parent_id: Some(1),
        };
        let res = client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(serde::to_string(&req_body).unwrap())
            .dispatch();
        assert_eq!(res.status(), Status::NotFound);
        let body: BasicMessage = res.into_json().unwrap();
        let expected = BasicMessage {
            message: String::from("No folder with the passed parentId was found."),
        };
        assert_eq!(body, expected);
    }

    #[test]
    fn update_folder_without_creds() {
        initialize_db().unwrap();
        remove_files();
        let client = client();
        let res = client.put(uri!("/folders")).dispatch();
        // without a password set
        assert_eq!(res.status(), Status::Unauthorized);
        // now with a password set
        set_password();
        let res = client.put(uri!("/folders")).dispatch();
        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[test]
    fn update_folder() {
        set_password();
        remove_files();
        let client = client();
        let create_request = serde::to_string(&CreateFolderRequest {
            name: String::from("test"),
            parent_id: None,
        })
        .unwrap();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(create_request)
            .dispatch();
        // folder should have id of 1 since it's the first one
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: None,
            name: String::from("testRenamed"),
            id: 1,
        })
        .unwrap();
        let res = client
            .put("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(update_request)
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let body: FolderResponse = res.into_json().unwrap();
        let expected = FolderResponse {
            id: 1,
            parent_id: None,
            path: String::from("testRenamed"),
            folders: Vec::new(),
            files: Vec::new(),
        };
        assert_eq!(body, expected);
    }

    #[test]
    fn update_folder_not_found() {
        set_password();
        remove_files();
        let client = client();
        let create_request = serde::to_string(&CreateFolderRequest {
            name: String::from("test"),
            parent_id: None,
        })
        .unwrap();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(create_request)
            .dispatch();
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: None,
            name: String::from("testRenamed"),
            id: 2,
        })
        .unwrap();
        let res = client
            .put("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(update_request)
            .dispatch();
        assert_eq!(res.status(), Status::NotFound);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(
            body,
            BasicMessage {
                message: String::from("The folder with the passed id could not be found.")
            }
        );
    }

    #[test]
    fn update_folder_parent_not_found() {
        set_password();
        remove_files();
        let client = client();
        let create_request = serde::to_string(&CreateFolderRequest {
            name: String::from("test"),
            parent_id: None,
        })
        .unwrap();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(create_request)
            .dispatch();
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(3),
            name: String::from("testRenamed"),
            id: 1,
        })
        .unwrap();
        let res = client
            .put("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(update_request)
            .dispatch();
        assert_eq!(res.status(), Status::NotFound);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(
            body,
            BasicMessage {
                message: String::from("The parent folder with the passed id could not be found.")
            }
        );
    }

    #[test]
    fn update_folder_already_exists() {
        set_password();
        remove_files();
        let client = client();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("test"),
                    parent_id: None,
                })
                .unwrap(),
            )
            .dispatch();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("test2"),
                    parent_id: None,
                })
                .unwrap(),
            )
            .dispatch();
        // rename to the second created folder
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: None,
            // windows is a case insensitive file system
            name: String::from("Test2"),
            id: 1,
        })
        .unwrap();
        let res = client
            .put("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(update_request)
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(
            body,
            BasicMessage {
                message: String::from(
                    "Cannot update folder, because another one with the new path already exists."
                )
            }
        );
    }

    #[test]
    fn update_folder_folder_already_exists_root() {
        set_password();
        remove_files();
        let client = client();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("test"),
                    parent_id: None,
                })
                .unwrap(),
            )
            .dispatch();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("test2"),
                    parent_id: Some(1),
                })
                .unwrap(),
            )
            .dispatch();
        // move the parent folder into the child
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(2),
            name: String::from("test"),
            id: 1,
        })
        .unwrap();
        let res = client
            .put("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(update_request)
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(
            body,
            BasicMessage {
                message: String::from("Cannot move parent folder into its own child.")
            }
        );
    }

    #[test]
    fn update_folder_folder_already_exists_target_folder() {
        set_password();
        remove_files();
        let client = client();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("test"),
                    parent_id: None,
                })
                .unwrap(),
            )
            .dispatch();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("test2"),
                    parent_id: Some(1),
                })
                .unwrap(),
            )
            .dispatch();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("test3"),
                    parent_id: Some(1),
                })
                .unwrap(),
            )
            .dispatch();
        // move the parent folder into the child
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(1),
            name: String::from("test3"),
            id: 2,
        })
        .unwrap();
        let res = client
            .put("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(update_request)
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(
            body,
            BasicMessage {
                message: String::from(
                    "Cannot update folder, because another one with the new path already exists."
                )
            }
        );
    }

    #[test]
    fn delete_folder_without_creds() {
        initialize_db().unwrap();
        remove_files();
        let client = client();
        let res = client.delete(uri!("/folders/1")).dispatch();
        // without a password set
        assert_eq!(res.status(), Status::Unauthorized);
        // now with a password set
        set_password();
        let res = client.delete(uri!("/folders/1")).dispatch();
        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[test]
    fn delete_folder() {
        set_password();
        remove_files();
        let client = client();
        // create a folder first to delete
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(
                serde::to_string(&CreateFolderRequest {
                    name: String::from("To Delete"),
                    parent_id: None,
                })
                .unwrap(),
            )
            .dispatch();
        // now delete the folder
        let delete_response = client
            .delete("/folders/1")
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        assert_eq!(delete_response.status(), Status::NoContent);
        // make sure the folder doesn't come back
        let get_folder_response = client
            .get("/folders/1")
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        assert_eq!(get_folder_response.status(), Status::NotFound);
    }

    #[test]
    fn delete_folder_should_not_delete_root() {
        set_password();
        remove_files();
        std::fs::create_dir(Path::new(FILE_DIR)).unwrap();
        let client = client();
        // make sure /null and /0 don't remove the files folder
        for id in ["null", "0"] {
            let res = client
                .delete(String::from("/") + id)
                .header(Header::new("Authorization", AUTH))
                .dispatch();
            assert_eq!(res.status(), Status::NotFound);
            assert!(Path::new("files").exists());
        }
    }

    #[test]
    fn delete_folder_not_found() {
        set_password();
        remove_files();
        let client = client();
        let response = client
            .delete("/folders/1")
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }
}

#[cfg(test)]
mod file_tests {
    use std::fs;
    use std::path::Path;

    use crate::model::repository::FileRecord;
    use rocket::http::{Header, Status};
    use rocket::local::blocking::Client;

    use crate::model::response::file_responses::FileMetadataResponse;
    use crate::model::response::BasicMessage;
    use crate::repository::{file_repository, initialize_db, open_connection};
    use crate::service::file_service::FILE_DIR;
    use crate::test::{refresh_db, remove_files, AUTH};

    use super::rocket;

    fn client() -> Client {
        Client::tracked(rocket()).unwrap()
    }

    fn fail() {
        assert!(false);
    }

    fn create_file_db_entry(name: &str) {
        let connection = open_connection();
        file_repository::create_file(
            &FileRecord {
                id: None,
                name: String::from(name),
            },
            &connection,
        )
        .unwrap();
        connection.close().unwrap();
    }

    fn create_file_disk(file_name: &str, contents: &str) {
        // TODO change the second () in OK to ! once it's no longer experimental (https://doc.rust-lang.org/std/primitive.never.html)
        fs::create_dir(Path::new("files"))
            .or_else(|_| Ok::<(), ()>(()))
            .unwrap();
        fs::write(
            Path::new(format!("{}/{}", FILE_DIR, file_name).as_str()),
            contents,
        )
        .unwrap();
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
    }

    #[test]
    fn get_file() {
        set_password();
        remove_files();
        // need to add to the database
        let connection = open_connection();
        file_repository::create_file(
            &FileRecord {
                id: None,
                name: String::from("file_name.txt"),
            },
            &connection,
        )
        .unwrap();
        connection.close().unwrap();
        let client = client();
        let res = client
            .get(uri!("/files/metadata/1"))
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        let status = res.status();
        let body: FileMetadataResponse = res.into_json().unwrap();
        assert_eq!(status, Status::Ok);
        assert_eq!(body.name, String::from("file_name.txt"));
        assert_eq!(body.id, 1);
    }

    #[test]
    fn search_files_without_creds() {
        refresh_db();
        remove_files();
        let client = client();
        let res = client.get(uri!("/files?search=test")).dispatch();
        // without a password set
        assert_eq!(res.status(), Status::Unauthorized);
        // now with a password set
        set_password();
        let res = client.get(uri!("/files?search=test")).dispatch();
        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[test]
    fn search_files_bad_search_query() {
        set_password();
        remove_files();
        let client = client();
        let res = client
            .get("/files?search")
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(body.message, String::from("Search string is required."));
    }

    #[test]
    fn search_files() {
        set_password();
        remove_files();
        // need to add to the database
        create_file_db_entry("should_return.txt");
        create_file_db_entry("should_not_return.txt");
        let client = client();
        let res = client
            .get("/files?search=should_return")
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let body: Vec<FileMetadataResponse> = res.into_json().unwrap();
        assert_eq!(body.len(), 1);
        let file = &body[0];
        assert_eq!(file.id, 1);
        assert_eq!(file.name, String::from("should_return.txt"));
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
    }

    #[test]
    fn download_file() {
        set_password();
        remove_files();
        create_file_db_entry("test.txt");
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
    }

    #[test]
    fn delete_file_without_creds() {
        fail()
    }

    #[test]
    fn delete_file_not_found() {
        fail()
    }

    #[test]
    fn delete_file() {
        fail()
    }

    #[test]
    fn update_file_without_creds() {
        fail()
    }

    #[test]
    fn update_file_file_not_found() {
        fail()
    }

    #[test]
    fn update_file_target_folder_not_found() {
        fail()
    }

    #[test]
    fn update_file_file_already_exists_root() {
        fail()
    }

    #[test]
    fn update_file_file_already_exists_target_folder() {
        fail()
    }

    #[test]
    fn update_file() {
        fail()
    }
}
