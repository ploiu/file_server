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

    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::BasicMessage;
    use crate::repository::initialize_db;
    use crate::test::{refresh_db, remove_files, AUTH};

    use super::rocket;

    fn set_password() {
        refresh_db();
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
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
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
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
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
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
        let client = Client::tracked(rocket()).expect("Valid Rocket Instance");
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
        assert!(false);
    }

    #[test]
    fn create_folder_non_existent() {
        assert!(false)
    }

    #[test]
    fn create_folder_already_exists() {
        assert!(false)
    }

    #[test]
    fn create_folder_parent_not_found() {
        assert!(false);
    }

    #[test]
    fn update_folder_without_creds() {
        assert!(false)
    }

    #[test]
    fn update_folder() {
        assert!(false)
    }

    #[test]
    fn update_folder_not_found() {
        assert!(false)
    }

    #[test]
    fn update_folder_parent_not_found() {
        assert!(false)
    }

    #[test]
    fn update_folder_already_exists() {
        assert!(false)
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
