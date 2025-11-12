use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::fs::{self};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::string::ToString;

use once_cell::sync::Lazy;
use regex::Regex;
use rocket::tokio::fs::create_dir;
use rusqlite::Connection;

use crate::model::api::FileApi;
use crate::model::error::file_errors::{
    CreateFileError, DeleteFileError, GetFileError, UpdateFileError,
};
use crate::model::error::folder_errors::{GetFolderError, LinkFolderError};
use crate::model::file_types::FileTypes;
use crate::model::repository::FileRecord;
use crate::model::request::file_requests::CreateFileRequest;
use crate::model::response::folder_responses::FolderResponse;
use crate::previews;
use crate::repository::{file_repository, folder_repository, open_connection};
use crate::service::{folder_service, tag_service};
use crate::{queue, repository};

/// mapping of file lowercase file extension => file type
static FILE_TYPE_MAPPING: Lazy<HashMap<&'static str, FileTypes>> = Lazy::new(|| {
    use FileTypes::*;
    HashMap::from([
        ("msi", Application),
        ("exe", Application),
        ("sh", Application),
        ("ps1", Application),
        ("bin", Application),
        ("jar", Application),
        ("bz2", Archive),
        ("gz", Archive),
        ("tar", Archive),
        ("bz", Archive),
        ("rar", Archive),
        ("zip", Archive),
        ("7z", Archive),
        ("midi", Audio),
        ("mp3", Audio),
        ("oga", Audio),
        ("ogg", Audio),
        ("opus", Audio),
        ("3g2", Audio),
        ("mid", Audio),
        ("3gp", Audio),
        ("wav", Audio),
        ("weba", Audio),
        ("ogx", Audio),
        ("aac", Audio),
        ("cda", Audio),
        ("flac", Audio),
        ("m4a", Audio),
        ("wma", Audio),
        ("f3d", Cad),
        ("php", Code),
        ("csh", Code),
        ("xml", Code),
        ("htm", Code),
        ("xhtml", Code),
        ("mjs", Code),
        ("jsonc", Code),
        ("jsonld", Code),
        ("json", Code),
        ("js", Code),
        ("html", Code),
        ("ts", Code),
        ("css", Code),
        ("py", Code),
        ("rs", Code),
        ("java", Code),
        ("c", Code),
        ("cpp", Code),
        ("h", Code),
        ("hpp", Code),
        ("go", Code),
        ("rb", Code),
        ("kt", Code),
        ("swift", Code),
        ("ini", Configuration),
        ("toml", Configuration),
        ("yml", Configuration),
        ("yaml", Configuration),
        ("properties", Configuration),
        ("conf", Configuration),
        ("config", Configuration),
        ("vsd", Diagram),
        ("rtf", Document),
        ("arc", Document),
        ("pdf", Document),
        ("doc", Document),
        ("odt", Document),
        ("epub", Document),
        ("abw", Document),
        ("md", Document),
        ("azw", Document),
        ("docx", Document),
        ("eot", Font),
        ("otf", Font),
        ("woff2", Font),
        ("ttf", Font),
        ("woff", Font),
        ("nds", Rom),
        ("wux", Rom),
        ("xci", Rom),
        ("nes", Rom),
        ("sfc", Rom),
        ("gb", Rom),
        ("gbc", Rom),
        ("gba", Rom),
        ("avif", Image),
        ("apng", Image),
        ("odg", Image),
        ("pdn", Image),
        ("bmp", Image),
        ("ico", Image),
        ("jpeg", Image),
        ("webp", Image),
        ("png", Image),
        ("svg", Image),
        ("jpg", Image),
        ("tif", Image),
        ("tiff", Image),
        ("gif", Image),
        ("heic", Image),
        ("heif", Image),
        ("mtl", Material),
        ("stl", Model),
        ("step", Model),
        ("stp", Model),
        ("fcstd", Model),
        ("3mf", Model),
        ("blend", Model),
        ("fbx", Model),
        ("gltf", Model),
        ("glb", Model),
        ("obj", Object),
        ("pptx", Presentation),
        ("ppt", Presentation),
        ("odp", Presentation),
        ("mcworld", SaveFile),
        ("sav", SaveFile),
        ("sgm", SaveFile),
        ("srm", SaveFile),
        ("xls", Spreadsheet),
        ("csv", Spreadsheet),
        ("ods", Spreadsheet),
        ("xlsx", Spreadsheet),
        ("txt", Text),
        ("log", Text),
        ("mpeg", Video),
        ("avi", Video),
        ("ogv", Video),
        ("mp4", Video),
        ("webm", Video),
        ("mov", Video),
        ("mkv", Video),
        ("flv", Video),
        ("wmv", Video),
        ("m4v", Video),
    ])
});

#[inline]
#[cfg(not(test))]
pub fn file_dir() -> String {
    "./files".to_string()
}

#[cfg(test)]
pub fn file_dir() -> String {
    let thread_name = crate::test::current_thread_name();
    let dir_name = format!("./{thread_name}");
    dir_name
}

/// ensures that the passed directory exists on the file system
pub async fn check_root_dir(dir: String) {
    let path = Path::new(dir.as_str());
    if !path.exists() {
        if let Err(e) = create_dir(path).await {
            panic!("Failed to create file directory: \n {e:?}")
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
    let resulting_file = if parent_id != 0 {
        // we requested a folder to put the file in, so make sure it exists
        let folder = folder_service::get_folder(Some(parent_id)).map_err(|e| {
            log::error!(
                "Save file - failed to retrieve parent folder. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            if e == GetFolderError::NotFound {
                CreateFileError::ParentFolderNotFound
            } else {
                CreateFileError::FailWriteDb
            }
        })?;
        // folder exists, now try to create the file
        let mut created =
            persist_save_file_to_folder(file_input, &folder, file_name.to_string()).await?;
        created.name = String::from(root_regex.replace(&file_name, ""));
        file_id = created.id.unwrap();
        created.into()
    } else {
        let file_extension = if let Some(ext) = &file_input.extension {
            format!(".{ext}")
        } else {
            String::from("")
        };
        let file_name = format!("{}/{}{}", &file_dir(), file_name, file_extension);
        let mut created = persist_save_file(file_input).await?;
        created.name = String::from(root_regex.replace(&file_name, ""));
        file_id = created.id.unwrap();
        created.into()
    };
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
                "Failed to pull file info from database. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
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
    Ok(FileApi::from_with_tags(file, tags))
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
            "Failed to delete file from disk at location {file_path:?}!\n Nested exception is {e:?}\n{}", Backtrace::force_capture()
        );
        DeleteFileError::FileSystemError
    })
}

/// uses an existing connection to delete file. Exists as an optimization to avoid having to open tons of repository connections when deleting a folder
pub fn delete_file_by_id_with_connection(id: u32, con: &Connection) -> Result<(), DeleteFileError> {
    // we first need to delete the file preview
    previews::delete_file_preview(id);
    let delete_result = file_repository::delete_file(id, con);
    match delete_result {
        Ok(_) => {}
        Err(e) => {
            if e == rusqlite::Error::QueryReturnedNoRows {
                log::warn!(
                    "attempted to delete a file with id {id} that does not exist in the database"
                );
            } else {
                log::error!(
                    "Failed to delete file record from database! Nested exception is {:?}\n{}",
                    e,
                    Backtrace::force_capture()
                );
                return Err(DeleteFileError::DbError);
            }
        }
    }
    Ok(())
}

pub fn update_file(file: FileApi) -> Result<FileApi, UpdateFileError> {
    let mut file = file;
    // first check if the file exists
    let con: Connection = repository::open_connection();
    let repo_file = file_repository::get_file(file.id, &con);
    if repo_file.is_err() {
        con.close().unwrap();
        return Err(UpdateFileError::NotFound);
    }
    let repo_file = repo_file.unwrap();
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
    // ensure file type gets updated if the name is changed
    file.file_type = Some(determine_file_type(&file.name));
    let converted_record = FileRecord::from(&file);
    if let Err(e) = file_repository::update_file(&converted_record, &con) {
        con.close().unwrap();
        log::error!(
            "Failed to update file record in database. Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
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
            "Failed to move file in the file system. Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(UpdateFileError::FileSystemError);
    }
    Ok(FileApi {
        id: file.id,
        folder_id: new_parent_id,
        name: file.name().unwrap(),
        tags,
        size: Some(repo_file.size),
        date_created: Some(repo_file.create_date),
        file_type: file.file_type,
    })
}

/// retrieves the full path to the file with the passed id
pub fn get_file_path(id: u32) -> Result<String, GetFileError> {
    let con = repository::open_connection();
    let result = file_repository::get_file_path(id, &con).map_err(|e| {
        log::error!(
            "Failed to get file path for file id {id}! Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        if e == rusqlite::Error::QueryReturnedNoRows {
            GetFileError::NotFound
        } else {
            GetFileError::DbFailure
        }
    });
    con.close().unwrap();
    result
}

/// looks at the passed `file_name`'s file extension and guesses which file type(s) are associated with that file.
pub fn determine_file_type(file_name: &str) -> FileTypes {
    let extension = Path::new(file_name).extension().and_then(OsStr::to_str);
    if let Some(ext) = extension {
        FILE_TYPE_MAPPING
            .get(ext.to_lowercase().as_str())
            .copied()
            .unwrap_or(FileTypes::Unknown)
    } else {
        // no extension means it's either a binary file or a text file. We _could_ read the file to determine,
        // but it looks like that can be tricky as we'd have to scan the entire file no matter how big and even then it might
        // not be guaranteed. I believe most of the time this would be text file...
        // but since there's no guarantee I'll leave it as unknown
        FileTypes::Unknown
    }
}

// ==== private functions ==== \\

/// persists the file to the disk and the database
async fn persist_save_file_to_folder(
    file_input: &mut CreateFileRequest<'_>,
    folder: &FolderResponse,
    file_name: String,
) -> Result<FileRecord, CreateFileError> {
    let file_name = determine_file_name(&file_name, &file_input.extension);
    let formatted_name = format!("{}/{}/{}", file_dir(), folder.path, file_name);
    match file_input.file.persist_to(&formatted_name).await {
        Ok(_) => {
            // path function here is guaranteed to return some at this point, according to docs
            let file_path = file_input.file.path().unwrap();
            let file_size = if let Ok(metadata) = fs::metadata(file_path) {
                metadata.size()
            } else {
                0
            };
            let res = save_file_record(&formatted_name, file_size)?;
            // file and folder are both in repository, now link them
            if link_folder_to_file(res.id.unwrap(), folder.id).is_err() {
                return Err(CreateFileError::FailWriteDb);
            }
            Ok(res)
        }
        Err(e) => {
            log::error!(
                "Failed to save file to disk. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            Err(CreateFileError::FailWriteDisk)
        }
    }
}

/// persists the passed file to the disk and the database
async fn persist_save_file(
    file_input: &mut CreateFileRequest<'_>,
) -> Result<FileRecord, CreateFileError> {
    let file_name = determine_file_name(file_input.file.name().unwrap(), &file_input.extension);
    let file_name = format!("{}/{}", &file_dir(), file_name);
    match file_input.file.persist_to(&file_name).await {
        Ok(()) => {
            // path function here is guaranteed to return some at this point, according to docs
            let file_path = file_input.file.path().unwrap();
            let file_size = if let Ok(metadata) = fs::metadata(file_path) {
                metadata.size()
            } else {
                0
            };
            save_file_record(&file_name, file_size)
        }
        Err(e) => {
            log::error!(
                "Failed to save file to disk. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            Err(CreateFileError::FailWriteDisk)
        }
    }
}

fn save_file_record(name: &str, size: u64) -> Result<FileRecord, CreateFileError> {
    // remove the './' from the file name
    let begin_path_regex = Regex::new("\\.?(/.*/)+?").unwrap();
    let formatted_name = begin_path_regex.replace(name, "");
    let file_type = determine_file_type(name);

    // Try to parse EXIF data for creation date if it's an image or video
    let create_date = match file_type {
        FileTypes::Image | FileTypes::Video => crate::exif::service::parse_exif_date(name)
            .unwrap_or_else(|| chrono::offset::Local::now().naive_local()),
        _ => chrono::offset::Local::now().naive_local(),
    };

    let mut file_record = FileRecord {
        id: None,
        name: formatted_name.to_string(),
        parent_id: None,
        create_date,
        size,
        file_type,
    };
    let con = repository::open_connection();
    let res =
        file_repository::create_file(&file_record, &con).map_err(|_| CreateFileError::FailWriteDb);
    con.close().unwrap();
    file_record.id = Some(res.unwrap());
    Ok(file_record)
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

#[cfg(test)]
mod save_file_record_tests {
    use super::*;
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn save_file_record_uses_exif_date_for_images() {
        init_db_folder();
        // Create a test image file (won't have real EXIF but will test the code path)
        let file_path = format!("{}/test.jpg", file_dir());
        std::fs::create_dir_all(file_dir()).unwrap();
        std::fs::write(&file_path, "fake image data").unwrap();

        let result = save_file_record(&file_path, 100);
        assert!(result.is_ok(), "Should successfully save file record");

        let record = result.unwrap();
        // Verify that the file type is Image
        assert_eq!(record.file_type, FileTypes::Image);
        // Verify that create_date is set (will be current date since no EXIF)
        assert!(record.create_date.timestamp() > 0);

        cleanup();
    }

    #[test]
    fn save_file_record_uses_current_date_for_non_images() {
        init_db_folder();
        // Create a test text file
        let file_path = format!("{}/test.txt", file_dir());
        std::fs::create_dir_all(file_dir()).unwrap();
        std::fs::write(&file_path, "test content").unwrap();

        let before_time = chrono::offset::Local::now().naive_local();
        let result = save_file_record(&file_path, 100);
        assert!(result.is_ok(), "Should successfully save file record");

        let record = result.unwrap();
        // Verify that the file type is Text
        assert_eq!(record.file_type, FileTypes::Text);
        // Verify that create_date is close to current time
        let diff = (record.create_date.timestamp() - before_time.timestamp()).abs();
        assert!(diff < 5, "Date should be within 5 seconds of current time");

        cleanup();
    }
}

/// checks the db to see if we have a record of the passed file
fn check_file_in_dir(
    file_input: &mut CreateFileRequest,
    file_name: &str,
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
fn determine_file_name(root_name: &str, extension: &Option<String>) -> String {
    if let Some(ext) = extension {
        format!("{root_name}.{ext}")
    } else {
        root_name.to_string()
    }
}

#[cfg(test)]
mod determine_file_name_tests {
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

#[cfg(test)]
mod update_file_tests {
    use std::fs;

    use crate::model::api::FileApi;
    use crate::model::error::file_errors::UpdateFileError;
    use crate::model::file_types::FileTypes;

    use crate::model::response::TagApi;
    use crate::model::response::folder_responses::FolderResponse;
    use crate::service::file_service::{file_dir, get_file_metadata, update_file};
    use crate::service::folder_service;
    use crate::test::{
        cleanup, create_file_db_entry, create_file_disk, create_folder_db_entry,
        create_folder_disk, create_tag_file, init_db_folder, now,
    };

    #[test]
    fn update_file_adds_tags() {
        init_db_folder();
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
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
        })
        .unwrap();
        let res = get_file_metadata(1).unwrap();
        assert_eq!(res.id, 1);
        assert_eq!(res.name, "test.txt".to_string());
        assert_eq!(res.folder_id, None);
        assert_eq!(
            res.tags,
            vec![TagApi {
                id: Some(1),
                title: "tag1".to_string(),
            }]
        );
        assert_eq!(res.file_type, Some(FileTypes::Text));
        assert_eq!(res.size, Some(0));
        cleanup();
    }

    #[test]
    fn update_file_removes_tags() {
        init_db_folder();
        create_file_db_entry("test.txt", None);
        create_file_disk("test.txt", "test");
        create_tag_file("tag1", 1);
        update_file(FileApi {
            id: 1,
            folder_id: Some(0),
            name: "test.txt".to_string(),
            tags: vec![],
            size: None,
            date_created: None,
            file_type: None,
        })
        .unwrap();
        let res = get_file_metadata(1).unwrap();
        assert_eq!(res.id, 1);
        assert_eq!(res.name, "test.txt".to_string());
        assert_eq!(res.folder_id, None);
        assert_eq!(res.tags, vec![]);
        assert_eq!(res.file_type, Some(FileTypes::Text));
        assert_eq!(res.size, Some(0));
        cleanup();
    }

    #[test]
    fn update_file_not_found() {
        init_db_folder();
        let res = update_file(FileApi {
            id: 1,
            folder_id: None,
            name: "test".to_string(),
            tags: vec![],
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::NotFound, res);
        cleanup();
    }

    #[test]
    fn update_file_target_folder_not_found() {
        init_db_folder();
        create_file_db_entry("test.txt", None);
        let res = update_file(FileApi {
            id: 1,
            name: "test.txt".to_string(),
            folder_id: Some(1),
            tags: vec![],
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FolderNotFound, res);
        cleanup();
    }

    #[test]
    fn update_file_file_already_exists_root() {
        init_db_folder();
        create_file_db_entry("test.txt", None);
        create_file_db_entry("test2.txt", None);
        create_file_disk("test.txt", "test");
        create_file_disk("test2.txt", "test2");
        let res = update_file(FileApi {
            id: 1,
            name: "test2.txt".to_string(),
            folder_id: None,
            tags: vec![],
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
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
        init_db_folder();
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
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
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
        init_db_folder();
        create_file_db_entry("test.txt", None);
        create_file_disk("test.txt", "test");
        update_file(FileApi {
            id: 1,
            name: "test".to_string(),
            folder_id: None,
            tags: vec![],
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
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
        init_db_folder();
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
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
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
        init_db_folder();
        create_folder_db_entry("test", None); // id 1
        create_file_db_entry("file", None); // id 1
        let res = update_file(FileApi {
            id: 1,
            folder_id: Some(0),
            name: "test".to_string(),
            tags: vec![],
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
        })
        .unwrap_err();
        assert_eq!(UpdateFileError::FolderAlreadyExistsWithSameName, res);
        // verify the database hasn't changed (file id 1 should be named file in root folder)
        let root_files = folder_service::get_folder(None).unwrap().files;
        assert_eq!(1, root_files.len());
        let file = &root_files[0];
        assert_eq!(file.id, 1);
        assert_eq!(file.name, "file".to_string());
        assert_eq!(file.folder_id, None);
        assert_eq!(file.tags, vec![]);
        assert_eq!(file.file_type, Some(FileTypes::Unknown));
        assert_eq!(file.size, Some(0));
        cleanup();
    }

    #[test]
    fn update_file_to_folder_with_same_name_same_folder() {
        init_db_folder();
        create_folder_db_entry("test", None); // folder id 1
        create_folder_db_entry("a", Some(1)); // folder id 2
        create_file_db_entry("file", None); // file id 1
        let res = update_file(FileApi {
            id: 1,
            name: "a".to_string(),
            folder_id: Some(1),
            tags: vec![],
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
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
        init_db_folder();
        create_folder_db_entry("test", None); // folder id 1
        create_folder_db_entry("a", Some(1)); // folder id 2
        create_file_db_entry("file", None); // file id 1; from root to folder id 1
        let res = update_file(FileApi {
            id: 1,
            name: "a".to_string(),
            folder_id: Some(1),
            tags: vec![],
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
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
        init_db_folder();
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
            size: Some(0),
            date_created: Some(now()),
            file_type: None,
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

    #[test]
    fn updates_file_type() {
        init_db_folder();
        create_file_db_entry("test", None);
        create_file_disk("test", "");
        let file = FileApi {
            id: 1,
            folder_id: None,
            name: "test.txt".to_string(),
            tags: vec![],
            size: None,
            date_created: None,
            file_type: Some(FileTypes::Text),
        };
        update_file(file).unwrap();
        let retrieved = get_file_metadata(1);
        assert_eq!(Some(FileTypes::Text), retrieved.unwrap().file_type);
        cleanup();
    }
}

#[cfg(test)]
mod delete_file_with_id_tests {
    use rocket::tokio;

    use crate::{
        model::error::file_errors,
        previews::get_file_preview,
        service::file_service::*,
        test::{cleanup, create_file_db_entry, create_file_preview, init_db_folder},
    };

    #[tokio::test]
    async fn test_deletes_file_properly() {
        init_db_folder();
        create_file_db_entry("test.txt", None);
        let con = open_connection();
        delete_file_by_id_with_connection(1, &con).unwrap();
        con.close().unwrap();
        let file = get_file_metadata(1).unwrap_err();
        assert_eq!(GetFileError::NotFound, file);
        cleanup();
    }

    #[tokio::test]
    async fn test_deletes_file_preview() {
        init_db_folder();
        create_file_db_entry("test.txt", None);
        create_file_preview(1);
        let con = open_connection();
        delete_file_by_id_with_connection(1, &con).unwrap();
        con.close().unwrap();
        let preview = get_file_preview(1).await.unwrap_err();
        assert_eq!(file_errors::GetPreviewError::NotFound, preview);
        cleanup();
    }
}

#[cfg(test)]
mod determine_file_type_tests {
    use super::*;

    #[test]
    fn test_step_file_is_model() {
        assert_eq!(determine_file_type("test.step"), FileTypes::Model);
        assert_eq!(determine_file_type("test.stp"), FileTypes::Model);
    }

    #[test]
    fn test_mov_file_is_video() {
        assert_eq!(determine_file_type("test.mov"), FileTypes::Video);
    }

    #[test]
    fn test_fcstd_file_is_model() {
        assert_eq!(determine_file_type("test.fcstd"), FileTypes::Model);
        assert_eq!(determine_file_type("test.FCStd"), FileTypes::Model);
    }

    #[test]
    fn test_3mf_file_is_model() {
        assert_eq!(determine_file_type("test.3mf"), FileTypes::Model);
    }

    #[test]
    fn test_conf_file_is_configuration() {
        assert_eq!(determine_file_type("test.conf"), FileTypes::Configuration);
        assert_eq!(determine_file_type("test.config"), FileTypes::Configuration);
    }

    #[test]
    fn test_additional_video_formats() {
        assert_eq!(determine_file_type("test.mkv"), FileTypes::Video);
        assert_eq!(determine_file_type("test.flv"), FileTypes::Video);
        assert_eq!(determine_file_type("test.wmv"), FileTypes::Video);
        assert_eq!(determine_file_type("test.m4v"), FileTypes::Video);
    }

    #[test]
    fn test_additional_audio_formats() {
        assert_eq!(determine_file_type("test.flac"), FileTypes::Audio);
        assert_eq!(determine_file_type("test.m4a"), FileTypes::Audio);
        assert_eq!(determine_file_type("test.wma"), FileTypes::Audio);
    }

    #[test]
    fn test_additional_code_formats() {
        assert_eq!(determine_file_type("test.py"), FileTypes::Code);
        assert_eq!(determine_file_type("test.rs"), FileTypes::Code);
        assert_eq!(determine_file_type("test.java"), FileTypes::Code);
        assert_eq!(determine_file_type("test.c"), FileTypes::Code);
        assert_eq!(determine_file_type("test.cpp"), FileTypes::Code);
        assert_eq!(determine_file_type("test.go"), FileTypes::Code);
    }

    #[test]
    fn test_additional_model_formats() {
        assert_eq!(determine_file_type("test.blend"), FileTypes::Model);
        assert_eq!(determine_file_type("test.fbx"), FileTypes::Model);
        assert_eq!(determine_file_type("test.gltf"), FileTypes::Model);
        assert_eq!(determine_file_type("test.glb"), FileTypes::Model);
    }

    #[test]
    fn test_additional_image_formats() {
        assert_eq!(determine_file_type("test.heic"), FileTypes::Image);
        assert_eq!(determine_file_type("test.heif"), FileTypes::Image);
    }

    #[test]
    fn test_yaml_configuration() {
        assert_eq!(determine_file_type("test.yaml"), FileTypes::Configuration);
    }

    #[test]
    fn test_log_text_file() {
        assert_eq!(determine_file_type("test.log"), FileTypes::Text);
    }

    #[test]
    fn test_unknown_extension() {
        assert_eq!(determine_file_type("test.xyz"), FileTypes::Unknown);
    }

    #[test]
    fn test_no_extension() {
        assert_eq!(determine_file_type("test"), FileTypes::Unknown);
    }
}
