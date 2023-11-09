use std::path::Path;

#[cfg(not(test))]
use rusqlite::OpenFlags;
use rusqlite::{Connection, Result};

pub mod file_repository;
pub mod folder_repository;
pub mod metadata_repository;
pub mod tag_repository;

#[cfg(not(test))]
static DB_LOCATION: &str = "./db.sqlite";

/// creates a new connection and returns it, but panics if the connection could not be created
#[cfg(not(test))]
pub fn open_connection() -> Connection {
    return match Connection::open_with_flags(Path::new(DB_LOCATION), OpenFlags::default()) {
        Ok(con) => con,
        Err(error) => panic!("Failed to get a connection to the database!: {}", error),
    };
}

#[cfg(test)]
pub fn open_connection() -> Connection {
    let db_name = format!("{}.sqlite", crate::test::current_thread_name());
    return match Connection::open_with_flags(
        Path::new(db_name.as_str()),
        rusqlite::OpenFlags::default(),
    ) {
        Ok(con) => con,
        Err(error) => panic!("Failed to get a connection to the database!: {}", error),
    };
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
    let table_version = match metadata_repository::get_version(&mut con) {
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

/// incrementally upgrades the database for each version the database is behind
fn migrate_db(con: &Connection, table_version: u64) -> Result<()> {
    if table_version < 2 {
        println!("Migrating database to v2...");
        migrate_v2(&con)?;
    }
    Ok(())
}

fn migrate_v2(con: &Connection) -> Result<()> {
    let migration_script = include_str!("../assets/migration/v2.sql");
    Ok(con.execute_batch(migration_script)?)
}
