use std::path::Path;

#[cfg(not(test))]
use rusqlite::OpenFlags;
use rusqlite::{Connection, Result};

use crate::db_migrations::migrate_db;

pub mod file_repository;
pub mod folder_repository;
pub mod metadata_repository;
pub mod tag_repository;

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
    // table_version will be used once we have more versions of the database
    let table_version = match metadata_repository::get_version(&con) {
        Ok(value) => value.parse::<u64>().unwrap(),
        Err(_) => {
            // tables haven't been created yet
            create_db(&mut con);
            1
        }
    };
    migrate_db(&con, table_version)?;
    con.close().unwrap();
    Ok(())
}
