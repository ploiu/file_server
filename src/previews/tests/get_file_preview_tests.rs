use rocket::tokio;

use crate::test::{cleanup, init_db_folder};

#[tokio::test]
#[cfg(not(ci))]
async fn should_return_preview_if_exists() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[tokio::test]
#[cfg(not(ci))]
async fn should_return_not_found_if_not_exists() {
    init_db_folder();
    crate::fail!();
    cleanup();
}
