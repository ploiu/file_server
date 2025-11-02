use crate::test::{cleanup, init_db_folder};

#[test]
fn should_return_preview_if_exists() {
    init_db_folder();
    crate::fail!();
    cleanup();
}

#[test]
fn should_return_not_found_if_not_exists() {
    init_db_folder();
    crate::fail!();
    cleanup();
}
