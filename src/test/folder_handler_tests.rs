use std::fs;
use std::path::{Path, PathBuf};

use rocket::http::{Header, Status};
use rocket::local::blocking::Client;
use rocket::serde::json::serde_json as serde;

use crate::model::file_types::FileTypes;
use crate::model::repository::{FileRecord, Folder};
use crate::model::request::folder_requests::{CreateFolderRequest, UpdateFolderRequest};
use crate::model::response::BasicMessage;
use crate::model::response::folder_responses::FolderResponse;
use crate::repository::{folder_repository, initialize_db, open_connection};
use crate::rocket;
use crate::service::file_service::file_dir;
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
fn get_root_folder() {
    set_password();
    remove_files();
    let client = client();
    let uri = uri!("/folders/metadata/null");
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
        tags: Vec::new(),
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
    let uri = uri!("/folders/metadata/0");
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
        tags: Vec::new(),
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
    let uri = uri!("/folders/metadata/1234");
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
    let res = client.get(uri!("/folders/metadata/1234")).dispatch();
    // without a password set
    assert_eq!(res.status(), Status::Unauthorized);
    // now with a password set
    set_password();
    let res = client.get(uri!("/folders/metadata/1234")).dispatch();
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
        tags: Vec::new(),
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
        tags: Vec::new(),
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
    create_folder_db_entry("test", None);
    create_folder_db_entry("test2", None);
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
    let root_files = folder_repository::get_child_files([], &con).unwrap_or(vec![]);
    assert_eq!(
        root_files[0],
        FileRecord {
            id: Some(1),
            name: String::from("file"),
            parent_id: None,
            size: 0,
            create_date: chrono::offset::Local::now().naive_local(),
            file_type: FileTypes::Unknown
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
    let root_folder = folder_repository::get_child_folders(Some(1), &con).unwrap_or_default();
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

#[test]
fn download_folder_returns_200_for_valid_folder() {
    set_password();
    remove_files();
    create_folder_disk("test/top/middle/bottom");
    create_folder_db_entry("test", None);
    create_folder_db_entry("top", Some(1));
    create_folder_db_entry("middle", Some(2));
    create_folder_db_entry("bottom", Some(3));
    create_file_db_entry("test", Some(4));
    create_file_disk("test/top/middle/bottom/test", "test");
    let client = client();
    let res = client
        .get(uri!("/folders/1"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(Status::Ok, res.status());
    cleanup();
}

#[test]
fn download_folder_returns_400_for_root() {
    set_password();
    remove_files();
    let client = client();
    let res = client
        .get(uri!("/folders/0"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(Status::BadRequest, res.status());
    cleanup();
}

#[test]
fn download_folder_returns_404_for_missing_id() {
    set_password();
    remove_files();
    let client = client();
    let res = client
        .get(uri!("/folders/12345"))
        .header(Header::new("Authorization", AUTH))
        .dispatch();
    assert_eq!(Status::NotFound, res.status());
    cleanup();
}
