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
    use rocket::http::{Header, Status};
    use rocket::local::blocking::Client;
    use rocket::serde::json::serde_json as serde;

    use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::BasicMessage;
    use crate::repository::initialize_db;
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
            name: String::from("test2"),
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
    fn update_folder_illegal_action() {
        // move parent into child
        assert!(false)
    }

    #[test]
    fn delete_folder_without_creds() {
        assert!(false)
    }

    #[test]
    fn delete_folder() {
        assert!(false)
    }

    #[test]
    fn delete_folder_not_found() {
        assert!(false)
    }
}
