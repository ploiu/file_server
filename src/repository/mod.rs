use std::fs;
use std::path::Path;

#[cfg(not(test))]
use rusqlite::OpenFlags;
use rusqlite::{Connection, Result};

use crate::db_migrations::migrate_db;
use crate::model::file_types::FileTypes;
use crate::model::repository::{FileRecord, Folder};
use crate::service::file_service::{determine_file_type, file_dir};

pub mod file_repository;
pub mod folder_repository;
pub mod metadata_repository;

#[cfg(test)]
mod tests;

/// creates a new connection and returns it, but panics if the connection could not be created
#[cfg(not(test))]
pub fn open_connection() -> Connection {
    use crate::config::FILE_SERVER_CONFIG;

    match Connection::open_with_flags(
        Path::new(FILE_SERVER_CONFIG.clone().database.location.as_str()),
        OpenFlags::default(),
    ) {
        Ok(con) => con,
        Err(error) => panic!("Failed to get a connection to the database!: {error}"),
    }
}

#[cfg(test)]
pub fn open_connection() -> Connection {
    let db_name = format!("{}.sqlite", crate::test::current_thread_name());
    match Connection::open_with_flags(Path::new(db_name.as_str()), rusqlite::OpenFlags::default()) {
        Ok(con) => con,
        Err(error) => panic!("Failed to get a connection to the database!: {error}"),
    }
}

/// runs init.sql on the database
fn create_db(con: &mut Connection) {
    let sql = include_str!("../assets/init.sql");
    con.execute_batch(sql).unwrap();
}

/// handles checking if the database exists and is up to the correct version.
/// If not, it either creates or upgrades the database accordingly
pub fn initialize_db() -> Result<()> {
    let mut con = open_connection();
    let mut should_gen_database_from_files = false;
    // table_version will be used once we have more versions of the database
    let table_version = match metadata_repository::get_version(&con) {
        Ok(value) => value.parse::<u64>().unwrap(),
        Err(_) => {
            // tables haven't been created yet
            create_db(&mut con);
            should_gen_database_from_files = true;
            1
        }
    };
    migrate_db(&con, table_version)?;
    if should_gen_database_from_files {
        generate_database_from_files(None, &con)?;
    }
    con.close().unwrap();
    Ok(())
}

/// Generates database entries from the existing files directory structure.
/// This walks the directory tree depth-first, creating folders before files at each level.
///
/// This function is designed to be called with `parent_folder = None` to start the
/// generation from the root files directory. The `parent_folder` parameter exists
/// to satisfy the API contract but the actual recursive traversal is handled internally.
///
/// # Arguments
/// * `parent_folder` - Should be None to start from root. Any other value is a no-op.
/// * `con` - Database connection
///
/// # Returns
/// * `Result<()>` - Ok if successful, or a rusqlite error
pub fn generate_database_from_files(parent_folder: Option<u32>, con: &Connection) -> Result<()> {
    // This function only processes the root level; recursion is handled internally
    if parent_folder.is_some() {
        return Ok(());
    }

    let base_path = file_dir();
    let path = Path::new(&base_path);
    if !path.exists() || !path.is_dir() {
        return Ok(());
    }

    // Check if directory is empty
    let entries: Vec<_> = match fs::read_dir(path) {
        Ok(iter) => iter.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    if entries.is_empty() {
        return Ok(());
    }

    generate_database_from_files_internal(&base_path, None, con)
}

/// Internal helper that walks the directory tree and creates database entries.
/// Walks depth-first, creating folders first at each level before files.
fn generate_database_from_files_internal(
    current_path: &str,
    parent_folder: Option<u32>,
    con: &Connection,
) -> Result<()> {
    let path = Path::new(current_path);

    let entries: Vec<_> = match fs::read_dir(path) {
        Ok(iter) => iter.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    // Separate folders and files
    let mut folders: Vec<_> = entries
        .iter()
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    let mut files: Vec<_> = entries
        .iter()
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .collect();

    // Sort for consistent ordering
    folders.sort_by_key(|e| e.file_name());
    files.sort_by_key(|e| e.file_name());

    // Process folders first (depth-first: process each folder fully before moving to next)
    for folder_entry in folders {
        let folder_name = folder_entry.file_name().to_string_lossy().to_string();

        // Create folder in database
        let folder = Folder {
            id: None,
            name: folder_name,
            parent_id: parent_folder,
        };

        let created_folder = folder_repository::create_folder(&folder, con)?;
        let folder_id = created_folder.id;

        // Recursively process this folder's contents (depth-first)
        let child_path = folder_entry.path();
        generate_database_from_files_internal(child_path.to_str().unwrap_or(""), folder_id, con)?;
    }

    // Then process files at this level
    for file_entry in files {
        let file_name = file_entry.file_name().to_string_lossy().to_string();
        let file_path = file_entry.path();

        // Get file size
        let file_size = fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

        // Determine file type
        let file_type: FileTypes = determine_file_type(&file_name);

        // Create file record
        let file_record = FileRecord {
            id: None,
            name: file_name,
            parent_id: parent_folder,
            create_date: chrono::offset::Local::now().naive_local(),
            size: file_size,
            file_type,
        };

        let file_id = file_repository::create_file(&file_record, con)?;

        // Link file to folder if not at root level
        if let Some(folder_id) = parent_folder {
            folder_repository::link_folder_to_file(file_id, folder_id, con)?;
        }
    }

    Ok(())
}
