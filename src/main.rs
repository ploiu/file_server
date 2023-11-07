#[macro_use]
extern crate rocket;

use std::fs;
use std::path::Path;

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
#[cfg(test)]
mod test;

#[cfg(not(test))]
fn temp_dir() -> String {
    return "./.file_server_temp".to_string();
}

#[cfg(test)]
fn temp_dir() -> String {
    let thread_name = test::current_thread_name();
    format!("./.{}_temp", thread_name)
}

#[launch]
fn rocket() -> Rocket<Build> {
    initialize_db().unwrap();
    fs::remove_dir_all(Path::new(temp_dir().as_str()))
        .or(Ok::<(), ()>(()))
        .unwrap();
    fs::create_dir(Path::new(temp_dir().as_str())).unwrap();
    fs::write("./.file_server_temp/.gitkeep", "").unwrap();
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
/// Look at .run/test.run.xml for run arguments - since there's ops on the same db file we need to run with 1 thread
/// TODO maybe change file directory for each test...that might make it so we can run tests in parallel
///
#[cfg(test)]
mod api_tests {
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    use crate::test::*;

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
    fn version() {
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
        let res = client.get(uri!("/api/version")).dispatch();
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_string().unwrap(), r#"{"version":"2.5.0"}"#);
        cleanup();
    }

    #[test]
    fn set_password_missing_fields() {
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
}

#[cfg(test)]
mod folder_tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use rocket::http::{Header, Status};
    use rocket::local::blocking::Client;
    use rocket::serde::json::serde_json as serde;

    use crate::model::repository::{FileRecord, Folder};
    use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::BasicMessage;
    use crate::repository::{file_repository, folder_repository, initialize_db, open_connection};
    use crate::service::file_service::file_dir;
    use crate::test::*;

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
            name: String::from("root"),
            folders: Vec::new(),
            files: Vec::new(),
        };
        let status = res.status();
        let res_json: FolderResponse = res.into_json().unwrap();
        assert_eq!(status, Status::Ok);
        assert_eq!(res_json, expected);
        cleanup();
    }
    #[test]
    fn get_root_folder_with_0_id() {
        set_password();
        remove_files();
        let client = client();
        let uri = uri!("/folders/0");
        let res = client
            .get(uri)
            .header(Header::new("Authorization", AUTH))
            .dispatch();
        let expected = FolderResponse {
            id: 0,
            parent_id: None,
            path: String::from("root"),
            name: String::from("root"),
            folders: Vec::new(),
            files: Vec::new(),
        };
        let status = res.status();
        let res_json: FolderResponse = res.into_json().unwrap();
        assert_eq!(status, Status::Ok);
        assert_eq!(res_json, expected);
        cleanup();
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
        cleanup();
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
        cleanup();
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
        cleanup();
    }
    #[test]
    fn create_folder_non_existent() {
        set_password();
        remove_files();
        let client = client();
        let req_body = CreateFolderRequest {
            name: String::from("whatever"),
            parent_id: Some(0),
        };
        let res = client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(serde::to_string(&req_body).unwrap())
            .dispatch();
        assert_eq!(res.status(), Status::Created);
        cleanup();
    }
    #[test]
    fn create_folder_parent_0_id() {
        set_password();
        remove_files();
        let client = client();
        let req_body = CreateFolderRequest {
            name: String::from("whatever"),
            parent_id: Some(0),
        };
        let res = client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(serde::to_string(&req_body).unwrap())
            .dispatch();
        assert_eq!(res.status(), Status::Created);
        cleanup();
    }
    #[test]
    fn create_folder_already_exists() {
        set_password();
        remove_files();
        let client = client();
        let req_body = CreateFolderRequest {
            name: String::from("whatever"),
            parent_id: Some(0),
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
        cleanup();
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
        cleanup();
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
        cleanup();
    }
    #[test]
    fn update_folder() {
        set_password();
        remove_files();
        let client = client();
        let create_request = serde::to_string(&CreateFolderRequest {
            name: String::from("test"),
            parent_id: Some(0),
        })
        .unwrap();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(create_request)
            .dispatch();
        // folder should have id of 1 since it's the first one
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(0),
            name: String::from("testRenamed"),
            id: 1,
            tags: Vec::new(),
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
            parent_id: Some(0),
            path: String::from("testRenamed"),
            name: String::from("testRenamed"),
            folders: Vec::new(),
            files: Vec::new(),
        };
        assert_eq!(body, expected);
        cleanup();
    }
    #[test]
    fn update_folder_new_folder_0_id() {
        set_password();
        remove_files();
        let client = client();
        let create_request = serde::to_string(&CreateFolderRequest {
            name: String::from("test"),
            parent_id: Some(0),
        })
        .unwrap();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(create_request)
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
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(0),
            name: String::from("testRenamed"),
            id: 2,
            tags: Vec::new(),
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
            id: 2,
            parent_id: Some(0),
            path: String::from("testRenamed"),
            name: String::from("testRenamed"),
            folders: Vec::new(),
            files: Vec::new(),
        };
        assert_eq!(body, expected);
        cleanup();
    }
    #[test]
    fn update_folder_not_found() {
        set_password();
        remove_files();
        let client = client();
        let create_request = serde::to_string(&CreateFolderRequest {
            name: String::from("test"),
            parent_id: Some(0),
        })
        .unwrap();
        client
            .post("/folders")
            .header(Header::new("Authorization", AUTH))
            .body(create_request)
            .dispatch();
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(0),
            name: String::from("testRenamed"),
            id: 2,
            tags: Vec::new(),
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
        cleanup();
    }
    #[test]
    fn update_folder_parent_not_found() {
        set_password();
        remove_files();
        let client = client();
        let create_request = serde::to_string(&CreateFolderRequest {
            name: String::from("test"),
            parent_id: Some(0),
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
            tags: Vec::new(),
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
        cleanup();
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
                    parent_id: Some(0),
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
                    parent_id: Some(0),
                })
                .unwrap(),
            )
            .dispatch();
        // rename to the second created folder
        let update_request = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(0),
            // windows is a case insensitive file system
            name: String::from("Test2"),
            id: 1,
            tags: Vec::new(),
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
        cleanup();
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
                    parent_id: Some(0),
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
            tags: Vec::new(),
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
        cleanup();
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
                    parent_id: Some(0),
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
            tags: Vec::new(),
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
        cleanup();
    }
    #[test]
    fn update_folder_root_not_found() {
        set_password();
        remove_files();
        let client = client();
        let body = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(0),
            name: String::from("test"),
            id: 0,
            tags: Vec::new(),
        })
        .unwrap();
        let res = client
            .put(uri!("/folders"))
            .header(Header::new("Authorization", AUTH))
            .body(body)
            .dispatch();
        assert_eq!(res.status(), Status::NotFound);
        cleanup();
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
        cleanup();
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
                    parent_id: Some(0),
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
        cleanup();
    }
    #[test]
    fn delete_folder_should_not_delete_root() {
        set_password();
        remove_files();
        std::fs::create_dir(Path::new(file_dir().as_str())).unwrap();
        let client = client();
        // make sure /null and /0 don't remove the files folder
        for id in ["null", "0"] {
            let res = client
                .delete(String::from("/") + id)
                .header(Header::new("Authorization", AUTH))
                .dispatch();
            assert_eq!(res.status(), Status::NotFound);
            let thread_name = current_thread_name();
            assert!(Path::new(thread_name.as_str()).exists());
        }
        cleanup();
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
        cleanup();
    }
    #[test]
    fn update_folder_to_file_with_same_name_root() {
        set_password();
        remove_files();
        create_folder_db_entry("test", None); // id 1
        create_folder_disk("test");
        create_file_db_entry("file", None); // id 1
        create_file_disk("file", "test");
        let client = client();
        let req = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(0),
            name: String::from("file"),
            id: 1,
            tags: Vec::new(),
        })
        .unwrap();
        let res = client
            .put(uri!("/folders"))
            .header(Header::new("Authorization", AUTH))
            .header(Header::new("Content-Type", "application/json"))
            .body(req)
            .dispatch();
        let status = res.status();
        let res_body: BasicMessage = res.into_json().unwrap();
        assert_eq!(status, Status::BadRequest);
        assert_eq!(res_body.message, "A file with that name already exists.");
        // verify the database hasn't changed (file id 1 should be named file in root folder)
        let con = open_connection();
        let root_files = folder_repository::get_child_files(None, &con).unwrap_or(vec![]);
        assert_eq!(
            root_files[0],
            FileRecord {
                id: Some(1),
                name: String::from("file"),
            }
        );
        let root_folders = folder_repository::get_child_folders(None, &con).unwrap();
        assert_eq!(root_folders.len(), 1);
        assert_eq!(
            root_folders[0],
            Folder {
                id: Some(1),
                name: String::from("test"),
                parent_id: None,
            }
        );
        con.close().unwrap();
        // verify the file system hasn't changed either
        let files: Vec<PathBuf> = fs::read_dir(file_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect::<Vec<_>>();
        assert_eq!(2, files.len());
        assert!(files.contains(&PathBuf::from(format!("{}/test", file_dir()))));
        assert!(files.contains(&PathBuf::from(format!("{}/file", file_dir()))));
        cleanup();
    }
    #[test]
    fn update_folder_to_file_with_same_name_same_folder() {
        set_password();
        remove_files();
        create_folder_db_entry("test", None); // folder id 1
        create_folder_disk("test");
        create_folder_db_entry("a", Some(1)); // folder id 2
        create_folder_disk("test/a");
        create_file_db_entry("file", Some(1)); // file id 1
        create_file_disk("file", "test");
        let client = client();
        let req = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(1),
            name: String::from("file"),
            id: 2,
            tags: Vec::new(),
        })
        .unwrap();
        let res = client
            .put(uri!("/folders"))
            .header(Header::new("Authorization", AUTH))
            .header(Header::new("Content-Type", "application/json"))
            .body(req)
            .dispatch();
        let status = res.status();
        let res_body: BasicMessage = res.into_json().unwrap();
        assert_eq!(status, Status::BadRequest);
        assert_eq!(res_body.message, "A file with that name already exists.");
        // verify the database hasn't changed (folder id 2 should be named a in test folder)
        let con = open_connection();
        let root_folders = folder_repository::get_child_folders(None, &con).unwrap();
        assert_eq!(root_folders.len(), 1);
        assert_eq!(
            root_folders[0],
            Folder {
                id: Some(1),
                name: String::from("test"),
                parent_id: None,
            }
        );
        let folder_1_folders = folder_repository::get_child_folders(Some(1), &con).unwrap();
        assert_eq!(folder_1_folders.len(), 1);
        assert_eq!(
            folder_1_folders[0],
            Folder {
                id: Some(2),
                name: String::from("test/a"),
                parent_id: Some(1),
            }
        );
        con.close().unwrap();
        /* verify the file system hasn't changed either
        ./files
            -> test
                -> a
            -> file
         */
        let folder_1_files: Vec<PathBuf> = fs::read_dir(format!("{}/{}", file_dir(), "test"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        let root_files: Vec<PathBuf> = fs::read_dir(file_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(1, folder_1_files.len());
        assert_eq!(2, root_files.len());
        assert!(folder_1_files.contains(&PathBuf::from(format!("{}/test/a", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/file", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/test", file_dir()))));
        cleanup();
    }
    #[test]
    fn update_folder_to_file_with_same_name_different_folder() {
        set_password();
        remove_files();
        /*
        ./files
            -> test
                -> a
            -> file
         */
        create_folder_db_entry("test", None); // folder id 1
        create_folder_disk("test");
        create_folder_db_entry("a", Some(1)); // folder id 2
        create_folder_disk("test/a");
        create_file_db_entry("file", None); // file id 1; from root to folder id 1
        create_file_disk("file", "test");
        let client = client();
        let req = serde::to_string(&UpdateFolderRequest {
            parent_id: Some(0),
            name: String::from("file"),
            id: 2,
            tags: Vec::new(),
        })
        .unwrap();
        let res = client
            .put(uri!("/folders"))
            .header(Header::new("Authorization", AUTH))
            .header(Header::new("Content-Type", "application/json"))
            .body(req)
            .dispatch();
        let status = res.status();
        let res_body: BasicMessage = res.into_json().unwrap();
        assert_eq!(status, Status::BadRequest);
        assert_eq!(res_body.message, "A file with that name already exists.");
        // verify the database hasn't changed (file id 1 should be named file in test folder)
        let con = open_connection();
        let root_folder = folder_repository::get_child_folders(Some(1), &con).unwrap_or(vec![]);
        con.close().unwrap();
        assert_eq!(
            root_folder[0],
            Folder {
                id: Some(2),
                name: String::from("test/a"),
                parent_id: Some(1),
            }
        );
        /* verify the file system hasn't changed either
        ./files
            -> test
                -> a
            -> file
         */
        let folder_1_files: Vec<PathBuf> = fs::read_dir(format!("{}/{}", file_dir(), "test"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect::<Vec<_>>();
        let root_files: Vec<PathBuf> = fs::read_dir(file_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(1, folder_1_files.len());
        assert_eq!(2, root_files.len());
        assert!(folder_1_files.contains(&PathBuf::from(format!("{}/test/a", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/file", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/test", file_dir()))));
        cleanup();
    }
}

#[cfg(test)]
mod file_tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use rocket::http::{Header, Status};
    use rocket::local::blocking::Client;
    use rocket::serde::json::serde_json as serde;

    use crate::model::repository::{FileRecord, Folder};
    use crate::model::request::file_requests::UpdateFileRequest;
    use crate::model::response::file_responses::FileMetadataResponse;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::BasicMessage;
    use crate::repository::{file_repository, folder_repository, initialize_db, open_connection};
    use crate::service::file_service::file_dir;
    use crate::test::*;

    use super::{rocket, test};

    fn client() -> Client {
        Client::tracked(rocket()).unwrap()
    }

    fn fail() {
        assert!(false);
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
        let res_body: FileMetadataResponse = res.into_json().unwrap();
        assert_eq!(
            res_body,
            FileMetadataResponse {
                id: 1,
                name: String::from("test.txt"),
            }
        );
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
        let res_body: FileMetadataResponse = res.into_json().unwrap();
        assert_eq!(
            res_body,
            FileMetadataResponse {
                id: 1,
                name: String::from("test"),
            }
        );
        // make sure that the file comes back with the right name
        let res: FileMetadataResponse = client
            .get(uri!("/files/metadata/1"))
            .header(Header::new("Authorization", AUTH))
            .dispatch()
            .into_json()
            .unwrap();
        assert_eq!(
            res,
            FileMetadataResponse {
                id: 1,
                name: String::from("test"),
            }
        );
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
        assert_eq!(body.message, String::from("Search string is required."));
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
        let body: Vec<FileMetadataResponse> = res.into_json().unwrap();
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
        match fs::read(format!("{}/{}", file_dir(), "test.txt")) {
            Ok(_) => fail(), // file still exists on disk
            Err(_) => { /* passed - no op */ }
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
    fn update_file_file_already_exists_root() {
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
        // now make sure that the files weren't changed on the disk
        let first = fs::read_to_string(format!("{}/{}", file_dir(), "test.txt")).unwrap();
        let second = fs::read_to_string(format!("{}/{}", file_dir(), "test2.txt")).unwrap();
        assert_eq!(first, String::from("test"));
        assert_eq!(second, String::from("test2"));
        cleanup();
    }
    #[test]
    fn update_file_file_already_exists_target_folder() {
        set_password();
        remove_files();
        test::create_folder_db_entry("test", None); // id 1
        test::create_folder_db_entry("target", None); // id 2
        create_folder_disk("test");
        create_folder_disk("target");
        // put the files in the folders
        test::create_file_db_entry("test.txt", Some(1)); // id 1
        test::create_file_db_entry("test.txt", Some(2)); // id 2
        create_file_disk("test/test.txt", "test");
        create_file_disk("target/test.txt", "target");
        // now try to move test/test.txt to target/test.txt
        let client = client();
        let res = client
            .put(uri!("/files"))
            .header(Header::new("Authorization", AUTH))
            //language=json
            .body(r#"{"id": 1, "name": "test.txt", "folderId": 2, "tags":  []}"#)
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        let body: BasicMessage = res.into_json().unwrap();
        assert_eq!(
            body.message,
            String::from("A file with the same name already exists in the specified folder")
        );
        // now make sure the file wasn't moved on the disk or db
        let get_first_res: String = client
            .get(uri!("/files/1"))
            .header(Header::new("Authorization", AUTH))
            .dispatch()
            .into_string()
            .unwrap();
        let get_second_res: String = client
            .get(uri!("/files/2"))
            .header(Header::new("Authorization", AUTH))
            .dispatch()
            .into_string()
            .unwrap();
        assert_eq!(get_first_res, "test");
        assert_eq!(get_second_res, "target");
        cleanup();
    }
    #[test]
    fn update_file_no_extension() {
        set_password();
        remove_files();
        test::create_file_db_entry("test.txt", None);
        create_file_disk("test.txt", "test");
        let client = client();
        let body =
            serde::to_string(&UpdateFileRequest::new(1, Some(0), "test".to_string())).unwrap();
        let res = client
            .put(uri!("/files"))
            .header(Header::new("Authorization", AUTH))
            .header(Header::new("Content-Type", "application/json"))
            .body(body)
            .dispatch();
        let status = res.status();
        assert_eq!(status, Status::Ok);
        let res_body: FileMetadataResponse = res.into_json().unwrap();
        assert_eq!(
            res_body,
            FileMetadataResponse {
                id: 1,
                name: String::from("test"),
            }
        );
        cleanup();
    }
    #[test]
    fn update_file() {
        set_password();
        remove_files();
        test::create_folder_db_entry("target_folder", None); // id 1
        test::create_file_db_entry("test.txt", None); // id 1
        test::create_file_db_entry("other.txt", Some(1)); // id 2
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
        let body: FileMetadataResponse = res.into_json().unwrap();
        assert_eq!(body.id, 1);
        assert_eq!(body.name, String::from("new_name.txt"));
        let folder_res: FolderResponse = client
            .get(uri!("/folders/1"))
            .header(Header::new("Authorization", AUTH))
            .dispatch()
            .into_json()
            .unwrap();
        assert_eq!(folder_res.files.len(), 2);
        cleanup();
    }
    #[test]
    fn update_file_to_folder_with_same_name_root() {
        set_password();
        remove_files();
        test::create_folder_db_entry("test", None); // id 1
        create_folder_disk("test");
        test::create_file_db_entry("file", None); // id 1
        create_file_disk("file", "test");
        let client = client();
        let req =
            serde::to_string(&UpdateFileRequest::new(1, Some(0), "test".to_string())).unwrap();
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
        // verify the database hasn't changed (file id 1 should be named file in root folder)
        let con = open_connection();
        let root_files = folder_repository::get_child_files(None, &con).unwrap_or(vec![]);
        con.close().unwrap();
        assert_eq!(
            root_files[0],
            FileRecord {
                id: Some(1),
                name: String::from("file"),
            }
        );
        // verify the file system hasn't changed either
        let files: Vec<PathBuf> = fs::read_dir(file_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect::<Vec<_>>();
        assert_eq!(2, files.len());
        assert!(files.contains(&PathBuf::from(format!("{}/test", file_dir()))));
        assert!(files.contains(&PathBuf::from(format!("{}/file", file_dir()))));
        cleanup();
    }
    #[test]
    fn update_file_to_folder_with_same_name_same_folder() {
        set_password();
        remove_files();
        test::create_folder_db_entry("test", None); // folder id 1
        create_folder_disk("test");
        test::create_folder_db_entry("a", Some(1)); // folder id 2
        create_folder_disk("test/a");
        test::create_file_db_entry("file", None); // file id 1
        create_file_disk("file", "test");
        let client = client();
        let req = serde::to_string(&UpdateFileRequest::new(1, Some(1), "a".to_string())).unwrap();
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
        // verify the database hasn't changed (file id 1 should be named file in test folder)
        let con = open_connection();
        let folder_1_db_files = folder_repository::get_child_files(Some(1), &con).unwrap();
        assert_eq!(folder_1_db_files.len(), 0);
        con.close().unwrap();
        // verify the file system hasn't changed either
        let folder_1_files: Vec<PathBuf> = fs::read_dir(format!("{}/{}", file_dir(), "test"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        let root_files: Vec<PathBuf> = fs::read_dir(file_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(1, folder_1_files.len());
        assert_eq!(2, root_files.len());
        assert!(folder_1_files.contains(&PathBuf::from(format!("{}/test/a", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/file", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/test", file_dir()))));
        cleanup();
    }
    #[test]
    fn update_file_to_folder_with_same_name_different_folder() {
        set_password();
        remove_files();
        test::create_folder_db_entry("test", None); // folder id 1
        create_folder_disk("test");
        test::create_folder_db_entry("a", Some(1)); // folder id 2
        create_folder_disk("test/a");
        test::create_file_db_entry("file", None); // file id 1; from root to folder id 1
        create_file_disk("file", "test");
        let client = client();
        let req = serde::to_string(&UpdateFileRequest::new(1, Some(1), "a".to_string())).unwrap();
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
        // verify the database hasn't changed (file id 1 should be named file in test folder)
        let con = open_connection();
        let root_folder = folder_repository::get_child_folders(Some(1), &con).unwrap_or(vec![]);
        con.close().unwrap();
        assert_eq!(
            root_folder[0],
            Folder {
                id: Some(2),
                name: String::from("test/a"),
                parent_id: Some(1),
            }
        );
        // verify the file system hasn't changed either
        let folder_1_files: Vec<PathBuf> = fs::read_dir(format!("{}/{}", file_dir(), "test"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect::<Vec<_>>();
        let root_files: Vec<PathBuf> = fs::read_dir(file_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(1, folder_1_files.len());
        assert_eq!(2, root_files.len());
        assert!(folder_1_files.contains(&PathBuf::from(format!("{}/test/a", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/file", file_dir()))));
        assert!(root_files.contains(&PathBuf::from(format!("{}/test", file_dir()))));
        cleanup();
    }

    #[test]
    fn test_update_file_trailing_name_fix() {
        set_password();
        remove_files();
        test::create_file_db_entry("test_thing.txt", None);
        create_file_disk("test_thing.txt", "test_thing");
        test::create_folder_db_entry("inner", None);
        create_folder_disk("inner");
        test::create_file_db_entry("thing.txt", Some(1));
        create_file_disk("inner/thing.txt", "thing");
        let client = client();
        let req =
            serde::to_string(&UpdateFileRequest::new(2, None, "thing.txt".to_string())).unwrap();
        let res = client
            .put(uri!("/files"))
            .header(Header::new("Authorization", AUTH))
            .header(Header::new("Content-Type", "application/json"))
            .body(req)
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        let body: FileMetadataResponse = res.into_json().unwrap();
        assert_eq!(body.id, 2);
        assert_eq!(body.name, String::from("thing.txt"));
        let folder_res: FolderResponse = client
            .get(uri!("/folders/0"))
            .header(Header::new("Authorization", AUTH))
            .dispatch()
            .into_json()
            .unwrap();
        assert_eq!(folder_res.files.len(), 2);
        cleanup();
    }
}
