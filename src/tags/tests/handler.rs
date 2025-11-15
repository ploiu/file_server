use rocket::http::{Header, Status};

use crate::model::response::TagApi;
use crate::repository::initialize_db;
use crate::test::*;

mod get_tag_tests {
    use super::*;

    #[test]
    fn without_creds() {
        initialize_db().unwrap();
        let client = client();
        let res = client.get(uri!("/tags/1")).dispatch();
        assert_eq!(res.status(), Status::Unauthorized);
        cleanup();
    }

    #[test]
    fn success() {
        set_password();
        create_tag_db_entry("test_tag");
        let client = client();
        let auth = Header::new("Authorization", AUTH);
        let res = client.get(uri!("/tags/1")).header(auth).dispatch();
        assert_eq!(res.status(), Status::Ok);
        cleanup();
    }

    #[test]
    fn not_found() {
        set_password();
        let client = client();
        let auth = Header::new("Authorization", AUTH);
        let res = client.get(uri!("/tags/999")).header(auth).dispatch();
        assert_eq!(res.status(), Status::NotFound);
        cleanup();
    }
}

mod create_tag_tests {
    use super::*;

    #[test]
    fn without_creds() {
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
    fn success() {
        set_password();
        let client = client();
        let auth = Header::new("Authorization", AUTH);
        let res = client
            .post(uri!("/tags"))
            .header(auth)
            .body(r#"{"title":"new_tag"}"#)
            .dispatch();
        assert_eq!(res.status(), Status::Created);
        cleanup();
    }
}

mod update_tag_tests {
    use super::*;

    #[test]
    fn without_creds() {
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
    fn success() {
        set_password();
        create_tag_db_entry("original_tag");
        let client = client();
        let auth = Header::new("Authorization", AUTH);
        let res = client
            .put(uri!("/tags"))
            .header(auth)
            .body(r#"{"id":1,"title":"updated_tag"}"#)
            .dispatch();
        assert_eq!(res.status(), Status::Ok);
        cleanup();
    }

    #[test]
    fn not_found() {
        set_password();
        let client = client();
        let auth = Header::new("Authorization", AUTH);
        let res = client
            .put(uri!("/tags"))
            .header(auth)
            .body(r#"{"id":999,"title":"updated_tag"}"#)
            .dispatch();
        assert_eq!(res.status(), Status::NotFound);
        cleanup();
    }

    #[test]
    fn already_exists() {
        set_password();
        create_tag_db_entry("tag1");
        create_tag_db_entry("tag2");
        let client = client();
        let auth = Header::new("Authorization", AUTH);
        let res = client
            .put(uri!("/tags"))
            .header(auth)
            .body(r#"{"id":2,"title":"tag1"}"#)
            .dispatch();
        assert_eq!(res.status(), Status::BadRequest);
        cleanup();
    }
}

mod delete_tag_tests {
    use super::*;

    #[test]
    fn without_creds() {
        initialize_db().unwrap();
        let client = client();
        let res = client.delete(uri!("/tags/1")).dispatch();
        assert_eq!(res.status(), Status::Unauthorized);
        cleanup();
    }

    #[test]
    fn success() {
        set_password();
        create_tag_db_entry("test_tag");
        let client = client();
        let auth = Header::new("Authorization", AUTH);
        let res = client.delete(uri!("/tags/1")).header(auth).dispatch();
        assert_eq!(res.status(), Status::NoContent);
        cleanup();
    }
}
