use rocket::tokio;

use crate::previews::get_previews_for_folder;
use crate::test::*;
use crate::test::{cleanup, init_db_folder};
use rocket::futures::StreamExt;

#[tokio::test]
async fn should_return_all_previews_for_found_files() {
    init_db_folder();
    create_file_db_entry("one.png", None);
    create_file_db_entry("two.png", None);
    create_file_preview(1);
    create_file_preview(2);

    let s = get_previews_for_folder(0).unwrap();
    let items: Vec<_> = s.collect().await;
    let ids: Vec<u32> = items.into_iter().map(|it| it.id).collect();
    assert_eq!(2, ids.len());
    assert!(ids.contains(&1));
    assert!(ids.contains(&2));
    cleanup();
}

#[tokio::test]
async fn should_return_nothing_for_files_if_no_preview() {
    init_db_folder();
    // create two files in the db but DO NOT create preview files for them
    create_file_db_entry("no_preview_one.png", None);
    create_file_db_entry("no_preview_two.png", None);

    // call the function under test for root folder
    let s = get_previews_for_folder(0).unwrap();
    let items: Vec<_> = s.collect().await;
    // No preview files exist, so the stream should be empty
    assert!(
        items.is_empty(),
        "expected no previews when none exist on disk"
    );

    cleanup();
}
