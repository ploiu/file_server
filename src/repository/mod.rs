use std::path::Path;

use rusqlite::{Connection, OpenFlags, Result};

pub mod file_repository;
pub mod folder_repository;
pub mod metadata_repository;

static DB_LOCATION: &str = "./db.sqlite";

/// creates a new connection and returns it, but panics if the connection could not be created
pub fn open_connection() -> Connection {
    return match Connection::open_with_flags(Path::new(DB_LOCATION), OpenFlags::default()) {
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
    let _table_version = match metadata_repository::get_version(&mut con) {
        Ok(value) => value.parse::<u64>().unwrap(),
        Err(_) => {
            // tables haven't been created yet
            create_db(&mut con);
            1
        }
    };
    con.close().unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn open_connection_creates_db_if_not_exist() {
        fs::remove_file(DB_LOCATION);
        open_connection().close().unwrap();
        let path = std::path::Path::new(DB_LOCATION);
        assert!(path.exists(), "db wasn't created!")
    }
}
