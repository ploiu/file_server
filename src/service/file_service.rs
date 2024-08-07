use std::fs;
use std::fs::File;
use std::path::Path;
use std::string::ToString;

use regex::Regex;
use rocket::tokio::fs::create_dir;
use rusqlite::Connection;

use crate::config::FILE_SERVER_CONFIG;
use crate::model::api::FileApi;
use crate::model::error::file_errors::{
    CreateFileError, DeleteFileError, GetFileError, GetPreviewError, UpdateFileError,
};
use crate::model::error::folder_errors::{GetFolderError, LinkFolderError};
use crate::model::repository::FileRecord;
use crate::model::request::file_requests::CreateFileRequest;
use crate::model::response::folder_responses::FolderResponse;
use crate::repository::{file_repository, folder_repository, metadata_repository, open_connection};
use crate::service::{folder_service, tag_service};
use crate::{queue, repository};

#[inline]
#[cfg(any(not(test), rust_analyzer))]
pub fn file_dir() -> String {
    "./files".to_string()
}

#[cfg(all(test, not(rust_analyzer)))]
pub fn file_dir() -> String {
    let thread_name = crate::test::current_thread_name();
    let dir_name = format!("./{}", thread_name);
    dir_name
}

/// ensures that the passed directory exists on the file system
pub async fn check_root_dir(dir: String) {
    let path = Path::new(dir.as_str());
    if !path.exists() {
        if let Err(e) = create_dir(path).await {
            panic!("Failed to create file directory: \n {:?}", e)
        }
    }
}

/// saves a file to the disk and database
pub async fn save_file(
    // because of this, we can't test this except through rocket
    file_input: &mut CreateFileRequest<'_>,
    force: bool,
) -> Result<FileApi, CreateFileError> {
    let file_name = String::from(file_input.file.name().unwrap());
    check_root_dir(file_dir()).await;
    if !force {
        check_file_in_dir(file_input, &file_name)?;
    }
    // we shouldn't leak implementation details to the client, so this strips the root dir from the response
    let root_regex = Regex::new(format!("^{}/", file_dir()).as_str()).unwrap();
    let parent_id = file_input.folder_id();
    let file_id: u32;
    let resulting_file: FileApi;
    if parent_id != 0 {
        // we requested a folder to put the file in, so make sure it exists
        let folder = folder_service::get_folder(Some(parent_id)).map_err(|e| {
            log::error!(
                "Save file - failed to retrieve parent folder. Nested exception is {:?}",
                e
            );
            if e == GetFolderError::NotFound {
                CreateFileError::ParentFolderNotFound
            } else {
                CreateFileError::FailWriteDb
            }
        })?;
        // folder exists, now try to create the file
        file_id =
            persist_save_file_to_folder(file_input, &folder, String::from(&file_name)).await?;
        resulting_file = FileApi {
            id: file_id,
            folder_id: None,
            name: String::from(root_regex.replace(&file_name, "")),
            tags: Vec::new(),
        }
    } else {
        let file_extension = if let Some(ext) = &file_input.extension {
            format!(".{}", ext)
        } else {
            String::from("")
        };
        let file_name = format!("{}/{}{}", &file_dir(), file_name, file_extension);
        file_id = persist_save_file(file_input).await?;
        resulting_file = FileApi {
            id: file_id,
            folder_id: None,
            name: String::from(root_regex.replace(&file_name, "")),
            tags: Vec::new(),
        }
    }
    // now publish the file to the rabbit queue so a preview can be generated for it later
    queue::publish_message("icon_gen", &file_id.to_string());
    Ok(resulting_file)
}

/// retrieves the file from the database with the passed id
pub fn get_file_metadata(id: u32) -> Result<FileApi, GetFileError> {
    let con: Connection = repository::open_connection();
    let file = match file_repository::get_file(id, &con) {
        Ok(f) => f,
        Err(e) => {
            con.close().unwrap();
            log::error!(
                "Failed to pull file info from database. Nested exception is {:?}",
                e
            );
            return if e == rusqlite::Error::QueryReturnedNoRows {
                Err(GetFileError::NotFound)
            } else {
                Err(GetFileError::DbFailure)
            };
        }
    };
    let tags = match tag_service::get_tags_on_file(id) {
        Ok(t) => t,
        Err(_) => {
            con.close().unwrap();
            return Err(GetFileError::TagError);
        }
    };
    con.close().unwrap();
    Ok(FileApi::from(file, tags))
}

pub fn check_file_exists(id: u32) -> bool {
    let con: Connection = open_connection();
    if file_repository::get_file(id, &con).is_err() {
        con.close().unwrap();
        return false;
    }
    con.close().unwrap();
    true
}

/// reads the contents of the file with the passed id from the disk and returns it
pub fn get_file_contents(id: u32) -> Result<File, GetFileError> {
    let res = get_file_path(id);
    if let Ok(path) = res {
        let path = format!("{}/{}", file_dir(), path);
        File::open(path).map_err(|_| GetFileError::NotFound)
    } else {
        Err(res.unwrap_err())
    }
}

pub fn delete_file(id: u32) -> Result<(), DeleteFileError> {
    let file_path = match get_file_path(id) {
        Ok(path) => format!("{}/{}", file_dir(), path),
        Err(GetFileError::NotFound) => return Err(DeleteFileError::NotFound),
        Err(_) => return Err(DeleteFileError::DbError),
    };
    // now that we've determined the file exists, we can remove from the repository
    let con = repository::open_connection();
    let delete_result = delete_file_by_id_with_connection(id, &con);
    con.close().unwrap();
    // helps avoid nested matches
    delete_result?;
    fs::remove_file(&file_path).map_err(|e| {
        log::error!(
            "Failed to delete file from disk at location {:?}!\n Nested exception is {:?}",
            file_path,
            e
        );
        DeleteFileError::FileSystemError
    })
}

/// uses an existing connection to delete file. Exists as an optimization to avoid having to open tons of repository connections when deleting a folder
pub fn delete_file_by_id_with_connection(id: u32, con: &Connection) -> Result<(), DeleteFileError> {
    // we first need to delete the file preview
    let preview_delete_result = file_repository::delete_file_preview(id, con);
    // we don't necessarily care if the preview wasn't deleted or not for this return value, but it's best to log it if error
    if preview_delete_result.is_err() {
        log::warn!(
            "Failed to delete file preview for file {id}. Exception is {:?}",
            preview_delete_result.unwrap_err()
        );
    }
    let delete_result = file_repository::delete_file(id, con);
    if delete_result.is_ok() {
        return Ok(());
    } else if Err(rusqlite::Error::QueryReturnedNoRows) == delete_result {
        return Err(DeleteFileError::NotFound);
    } else {
        log::error!(
            "Failed to delete file record from database! Nested exception is: \n {:?}",
            delete_result.unwrap_err()
        );
        return Err(DeleteFileError::DbError);
    }
}

pub fn update_file(file: FileApi) -> Result<FileApi, UpdateFileError> {
    // first check if the file exists
    let con: Connection = repository::open_connection();
    if file_repository::get_file(file.id, &con).is_err() {
        con.close().unwrap();
        return Err(UpdateFileError::NotFound);
    }
    // now check if the folder exists
    let parent_folder =
        folder_service::get_folder(file.folder_id).map_err(|_| UpdateFileError::FolderNotFound)?;
    // now check if a file with the passed name is already under that folder
    let name_regex = Regex::new(format!("^{}$", file.name().unwrap()).as_str()).unwrap();
    for f in parent_folder.files.iter() {
        // make sure to ignore name collision if the file with the same name is the exact same file
        if f.id != file.id && name_regex.is_match(f.name.as_str()) {
            return Err(UpdateFileError::FileAlreadyExists);
        }
    }
    for f in parent_folder.folders.iter() {
        if name_regex.is_match(f.name.as_str()) {
            return Err(UpdateFileError::FolderAlreadyExistsWithSameName);
        }
    }
    // we have to create this before we update the file
    let old_path = format!(
        "{}/{}",
        file_dir(),
        file_repository::get_file_path(file.id, &con).unwrap()
    );
    // now that we've verified that the file & folder exist and we're not gonna collide names, perform the move
    let new_parent_id = if file.folder_id == Some(0) {
        None
    } else {
        file.folder_id
    };
    if let Err(e) =
        file_repository::update_file(&file.id, &new_parent_id, &file.name().unwrap(), &con)
    {
        con.close().unwrap();
        log::error!(
            "Failed to update file record in database. Nested exception is {:?}",
            e
        );
        return Err(UpdateFileError::DbError);
    }
    // now that we've updated the file in the database, it's time to update the file system
    let new_path = format!(
        "{}/{}/{}",
        file_dir(),
        parent_folder.path,
        file.name().unwrap()
    );
    // update the file's tags in the db
    if tag_service::update_file_tags(file.id, file.tags.clone()).is_err() {
        con.close().unwrap();
        return Err(UpdateFileError::TagError);
    }
    let tags = match tag_service::get_tags_on_file(file.id) {
        Ok(t) => t,
        Err(_) => {
            con.close().unwrap();
            return Err(UpdateFileError::TagError);
        }
    };
    // we're done with the database for now
    con.close().unwrap();
    let new_path = Regex::new("/root").unwrap().replace(new_path.as_str(), "");
    if let Err(e) = fs::rename(old_path, new_path.to_string()) {
        log::error!(
            "Failed to move file in the file system. Nested exception is {:?}",
            e
        );
        return Err(UpdateFileError::FileSystemError);
    }
    Ok(FileApi {
        id: file.id,
        folder_id: new_parent_id,
        name: file.name().unwrap(),
        tags,
    })
}

/// retrieves the full path to the file with the passed id
pub fn get_file_path(id: u32) -> Result<String, GetFileError> {
    let con = repository::open_connection();
    let result = file_repository::get_file_path(id, &con).map_err(|e| {
        log::error!("Failed to get file path! Nested exception is {:?}", e);
        if e == rusqlite::Error::QueryReturnedNoRows {
            GetFileError::NotFound
        } else {
            GetFileError::DbFailure
        }
    });
    con.close().unwrap();
    result
}

/// Retrieves the preview contents of the file with the passed id in png format.
/// The preview might not immediately exist in the database at the time this function is called,
/// so extra care needs to be taken to not blow up if (when) that happens.
///
/// # Errors
///
/// This function will return an error if the preview doesn't exist in the database, or if the database fails. Regardless, a log will be emitted
pub fn get_file_preview(id: u32) -> Result<Vec<u8>, GetPreviewError> {
    let con: Connection = repository::open_connection();
    let result = file_repository::get_file_preview(id, &con).map_err(|e| {
        log::error!("Failed to get file preview! Nested exception is {:?}", e);
        if e == rusqlite::Error::QueryReturnedNoRows {
            GetPreviewError::NotFound
        } else {
            GetPreviewError::DbFailure
        }
    });
    con.close().unwrap();
    result
}

/// checks the database and generates previews for all files if the database doesn't have the flag `generated_previews` in the metadata table
pub fn generate_all_previews() {
    if !FILE_SERVER_CONFIG.clone().rabbit_mq.enabled {
        return;
    }
    log::info!("Starting to generate previews for existing files...");
    let con: Connection = open_connection();
    let flag_res = metadata_repository::get_generated_previews_flag(&con);
    if Ok(false) == flag_res {
        let file_ids = match file_repository::get_all_file_ids(&con) {
            Ok(ids) => ids,
            Err(e) => {
                con.close().unwrap();
                log::error!("Failed to retrieve all file IDs in the database. Error is {e:?}");
                return;
            }
        };
        for id in file_ids {
            queue::publish_message("icon_gen", &id.to_string());
        }
        let flag_set_result = metadata_repository::set_generated_previews_flag(&con);
        con.close().unwrap();
        if let Err(e) = flag_set_result {
            log::error!("Failed to set preview flag in database. Exception is {e:?}");
        } else {
            log::info!("Successfully pushed file IDs to queue")
        }
    } else if let Err(e) = flag_res {
        log::error!("Failed to get preview flag from database. Error is {e:?}");
        con.close().unwrap();
        return;
    } else {
        log::info!("Not generating file previews because the db flag is already set.")
    }
}

// ==== private functions ==== \\

/// persists the file to the disk and the database
async fn persist_save_file_to_folder(
    file_input: &mut CreateFileRequest<'_>,
    folder: &FolderResponse,
    file_name: String,
) -> Result<u32, CreateFileError> {
    let file_name = determine_file_name(&file_name, &file_input.extension);
    let formatted_name = format!("{}/{}/{}", file_dir(), folder.path, file_name);
    match file_input.file.persist_to(&formatted_name).await {
        Ok(_) => {
            let id = save_file_record(&formatted_name)?;
            // file and folder are both in repository, now link them
            if link_folder_to_file(id, folder.id).is_err() {
                return Err(CreateFileError::FailWriteDb);
            }
            Ok(id)
        }
        Err(e) => {
            log::error!("Failed to save file to disk. Nested exception is {:?}", e);
            Err(CreateFileError::FailWriteDisk)
        }
    }
}

/// persists the passed file to the disk and the database
async fn persist_save_file(file_input: &mut CreateFileRequest<'_>) -> Result<u32, CreateFileError> {
    let file_name = determine_file_name(
        &String::from(file_input.file.name().unwrap()),
        &file_input.extension,
    );
    let file_name = format!("{}/{}", &file_dir(), file_name);
    match file_input.file.persist_to(&file_name).await {
        Ok(_) => Ok(save_file_record(&file_name)?),
        Err(e) => {
            log::error!("Failed to save file to disk. Nested exception is {:?}", e);
            Err(CreateFileError::FailWriteDisk)
        }
    }
}

fn save_file_record(name: &str) -> Result<u32, CreateFileError> {
    // remove the './' from the file name
    let begin_path_regex = Regex::new("\\.?(/.*/)+?").unwrap();
    let formatted_name = begin_path_regex.replace(name, "");
    let file_record = FileRecord::from(formatted_name.to_string());
    let con = repository::open_connection();
    let res =
        file_repository::create_file(&file_record, &con).map_err(|_| CreateFileError::FailWriteDb);
    con.close().unwrap();
    res
}

/// adds a link to the folder for the passed file in the database
fn link_folder_to_file(file_id: u32, folder_id: u32) -> Result<(), LinkFolderError> {
    let con = repository::open_connection();
    let link_result = folder_repository::link_folder_to_file(file_id, folder_id, &con);
    con.close().unwrap();
    if link_result.is_err() {
        return Err(LinkFolderError::DbError);
    }
    Ok(())
}

/// checks the db to see if we have a record of the passed file
fn check_file_in_dir(
    file_input: &mut CreateFileRequest,
    file_name: &String,
) -> Result<(), CreateFileError> {
    log::warn!("{file_name}{:?}", &file_input.extension);
    let full_file_name = determine_file_name(file_name, &file_input.extension);
    // first check that the db does not have a record of the file in its directory
    let con = repository::open_connection();
    let db_parent_id = if 0 == file_input.folder_id() {
        vec![]
    } else {
        vec![file_input.folder_id()]
    };
    let child_files = folder_repository::get_child_files(db_parent_id, &con);
    con.close().unwrap();
    if child_files.is_err() {
        return Err(CreateFileError::FailWriteDb);
    }
    // compare the names of all the child files
    for child in child_files.unwrap().iter() {
        if child.name.to_lowercase() == full_file_name.to_lowercase() {
            log::warn!("Not saving file {full_file_name} because it already exists.");
            return Err(CreateFileError::AlreadyExists);
        }
    }
    Ok(())
}

/// Creates the file name based on whether or not the extension exists
/// Example:
/// ```
/// let root_name = String::from("test");
/// let extension = Some(String::from("txt"));
/// let file_name = determine_file_name(&root_name, &extension);
/// assert_eq!(file_name, String::from("test.txt"));
/// ```
fn determine_file_name(root_name: &String, extension: &Option<String>) -> String {
    if let Some(ext) = extension {
        format!("{}.{}", root_name, ext)
    } else {
        root_name.to_string()
    }
}

#[cfg(all(test, not(rust_analyzer)))]
mod deterine_file_name_tests {
    use super::determine_file_name;

    #[test]
    fn determine_file_name_with_ext() {
        let root_name = String::from("test");
        let extension = Some(String::from("txt"));
        let file_name = determine_file_name(&root_name, &extension);
        assert_eq!(file_name, String::from("test.txt"));
    }

    #[test]
    fn determine_file_name_without_ext() {
        let root_name = String::from("test");
        let extension = None;
        let file_name = determine_file_name(&root_name, &extension);
        assert_eq!(file_name, String::from("test"));
    }
}

#[cfg(all(test, not(rust_analyzer)))]
mod update_file_tests {
    use std::fs;

    use crate::model::api::FileApi;
    use crate::model::error::file_errors::UpdateFileError;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::model::response::TagApi;
    use crate::service::file_service::{file_dir, get_file_metadata, update_file};
    use crate::service::folder_service;
    use crate::test::{
        cleanup, create_file_db_entry, create_file_disk, create_folder_db_entry,
        create_folder_disk, create_tag_file, refresh_db,
    };

    #[test]
    fn update_file_adds_tags() {
        refresh_db();
        create_file_db_entry("test.txt", None);
        create_file_disk("test.txt", "test");
        update_file(FileApi {
            id: 1,
            folder_id: Some(0),
            name: "test.txt".to_string(),
            tags: vec![TagApi {
                id: None,
                title: "tag1".to_string(),
            }],
        })
        .unwrap();
        let res = get_file_metadata(1).unwrap();
        assert_eq!(
            FileApi {
                id: 1,
                folder_id: None,
                name: "test.txt".to_string(),
                tags: vec![TagApi {
                    id: Some(1),
                    title: "tag1".to_string(),
                }],
            },
            res
        );
        cleanup();
    }

    #[test]
    fn update_file_removes_tags() {
        refresh_db();
        create_file_db_entry("test.txt", None);
        create_file_disk("test.txt", "test");
        create_tag_file("tag1", 1);
        update_file(FileApi {
            id: 1,
            folder_id: Some(0),
            name: "test.txt".to_string(),
            tags: vec![],
        })
        .unwrap();
        let res = get_file_metadata(1).unwrap();
        assert_eq!(
            FileApi {
                id: 1,
                folder_id: None,
                name: "test.txt".to_string(),
                tags: vec![],
            },
            res
        );
        cleanup();
    }

    #[test]
    fn update_file_not_found() {
        refresh_db();
        let res = update_file(FileApi {
            id: 1,
            folder_id: None,
            name: "test".to_string(),
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::NotFound, res);
        cleanup();
    }

    #[test]
    fn update_file_target_folder_not_found() {
        refresh_db();
        create_file_db_entry("test.txt", None);
        let res = update_file(FileApi {
            id: 1,
            name: "test.txt".to_string(),
            folder_id: Some(1),
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FolderNotFound, res);
        cleanup();
    }

    #[test]
    fn update_file_file_already_exists_root() {
        refresh_db();
        create_file_db_entry("test.txt", None);
        create_file_db_entry("test2.txt", None);
        create_file_disk("test.txt", "test");
        create_file_disk("test2.txt", "test2");
        let res = update_file(FileApi {
            id: 1,
            name: "test2.txt".to_string(),
            folder_id: None,
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FileAlreadyExists, res);
        // now make sure that the files weren't changed on the disk
        let first = fs::read_to_string(format!("{}/{}", file_dir(), "test.txt")).unwrap();
        let second = fs::read_to_string(format!("{}/{}", file_dir(), "test2.txt")).unwrap();
        assert_eq!(first, String::from("test"));
        assert_eq!(second, String::from("test2"));
        cleanup();
    }

    #[test]
    fn update_file_file_already_exists_target_folder() {
        refresh_db();
        create_folder_db_entry("test", None); // id 1
        create_folder_db_entry("target", None); // id 2
                                                // put the files in the folders
        create_file_db_entry("test.txt", Some(1)); // id 1
        create_file_db_entry("test.txt", Some(2)); // id 2
        let res = update_file(FileApi {
            id: 1,
            name: "test.txt".to_string(),
            folder_id: Some(2),
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FileAlreadyExists, res);
        // make sure the file wasn't moved in the db
        let db_test_folder = folder_service::get_folder(Some(1)).unwrap();
        assert_eq!(db_test_folder.files[0].id, 1);
        let db_target_folder = folder_service::get_folder(Some(2)).unwrap();
        assert_eq!(db_target_folder.files[0].id, 2);
        cleanup();
    }

    #[test]
    fn update_file_no_extension() {
        refresh_db();
        create_file_db_entry("test.txt", None);
        create_file_disk("test.txt", "test");
        update_file(FileApi {
            id: 1,
            name: "test".to_string(),
            folder_id: None,
            tags: vec![],
        })
        .unwrap();
        let res = get_file_metadata(1).unwrap();
        assert_eq!("test".to_string(), res.name);
        // make sure the file is properly renamed on disk
        let file_contents = fs::read_to_string(format!("{}/test", file_dir())).unwrap();
        assert_eq!("test", file_contents);
        cleanup();
    }

    #[test]
    fn update_file_works() {
        refresh_db();
        create_folder_db_entry("target_folder", None); // id 1
        create_file_db_entry("test.txt", None); // id 1
        create_file_db_entry("other.txt", Some(1)); // id 2
        create_file_disk("test.txt", "test"); // (1)
        create_folder_disk("target_folder"); // (1)
        create_file_disk("target_folder/other.txt", "other"); // (2)
        let res = update_file(FileApi {
            id: 1,
            name: "new_name.txt".to_string(),
            folder_id: Some(1),
            tags: vec![],
        })
        .unwrap();
        assert_eq!(1, res.id);
        assert_eq!("new_name.txt", res.name);
        let containing_folder = folder_service::get_folder(Some(1)).unwrap();
        assert_eq!(2, containing_folder.files.len());
        cleanup();
    }

    #[test]
    fn update_file_to_folder_with_same_name_root() {
        refresh_db();
        create_folder_db_entry("test", None); // id 1
        create_file_db_entry("file", None); // id 1
        let res = update_file(FileApi {
            id: 1,
            folder_id: Some(0),
            name: "test".to_string(),
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FolderAlreadyExistsWithSameName, res);
        // verify the database hasn't changed (file id 1 should be named file in root folder)
        let root_files = folder_service::get_folder(None).unwrap().files;
        assert_eq!(
            root_files[0],
            FileApi {
                id: 1,
                name: String::from("file"),
                folder_id: None,
                tags: vec![],
            }
        );
        cleanup();
    }

    #[test]
    fn update_file_to_folder_with_same_name_same_folder() {
        refresh_db();
        create_folder_db_entry("test", None); // folder id 1
        create_folder_db_entry("a", Some(1)); // folder id 2
        create_file_db_entry("file", None); // file id 1
        let res = update_file(FileApi {
            id: 1,
            name: "a".to_string(),
            folder_id: Some(1),
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FolderAlreadyExistsWithSameName, res);
        // verify the db hasn't changed
        let folder_1_db_files = folder_service::get_folder(Some(1)).unwrap().files;
        assert_eq!(folder_1_db_files.len(), 0);
        cleanup();
    }

    #[test]
    fn update_file_to_folder_with_same_name_different_folder() {
        refresh_db();
        create_folder_db_entry("test", None); // folder id 1
        create_folder_db_entry("a", Some(1)); // folder id 2
        create_file_db_entry("file", None); // file id 1; from root to folder id 1
        let res = update_file(FileApi {
            id: 1,
            name: "a".to_string(),
            folder_id: Some(1),
            tags: vec![],
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FolderAlreadyExistsWithSameName, res);
        // verify the database hasn't changed (file id 1 should be named file in test folder)
        let root_folder = folder_service::get_folder(Some(1)).unwrap().folders;
        assert_eq!(
            root_folder[0],
            FolderResponse {
                id: 2,
                name: String::from("a"),
                folders: vec![],
                parent_id: Some(1),
                tags: vec![],
                path: "test/a".to_string(),
                files: vec![],
            }
        );
        cleanup();
    }

    #[test]
    fn update_file_trailing_name_fix() {
        refresh_db();
        create_file_db_entry("test_thing.txt", None);
        create_file_disk("test_thing.txt", "test_thing");
        create_folder_db_entry("inner", None);
        create_folder_disk("inner");
        create_file_db_entry("thing.txt", Some(1));
        create_file_disk("inner/thing.txt", "thing");
        update_file(FileApi {
            id: 2,
            name: "thing.txt".to_string(),
            folder_id: None,
            tags: vec![],
        })
        .unwrap();
        let folder_files = folder_service::get_folder(Some(0)).unwrap().files;
        assert_eq!(2, folder_files.len());
        let mut file_names: Vec<String> = fs::read_dir(file_dir())
            .unwrap()
            .map(|d| d.unwrap().file_name().into_string().unwrap())
            .collect();
        file_names.sort();
        assert_eq!(vec!["inner", "test_thing.txt", "thing.txt"], file_names);
        cleanup();
    }
}

#[cfg(all(test, not(rust_analyzer)))]
mod delete_file_with_id_tests {
    use crate::{
        service::file_service::*,
        test::{cleanup, create_file_db_entry, create_file_preview, refresh_db},
    };

    #[test]
    fn test_deletes_file_properly() {
        refresh_db();
        create_file_db_entry("test.txt", None);
        let con = open_connection();
        let res = delete_file_by_id_with_connection(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!((), res);
        let file = get_file_metadata(1).unwrap_err();
        assert_eq!(GetFileError::NotFound, file);
        cleanup();
    }

    #[test]
    fn test_deletes_file_preview() {
        refresh_db();
        create_file_db_entry("test.txt", None);
        create_file_preview(1);
        let con = open_connection();
        let res = delete_file_by_id_with_connection(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!((), res);
        let preview = get_file_preview(1).unwrap_err();
        assert_eq!(GetPreviewError::NotFound, preview);
        cleanup();
    }
}
