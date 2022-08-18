use rusqlite::{Connection, OpenFlags, Result};
use std::path::Path;

static DB_LOCATION: &str = "./db.sqlite";

fn open_connection() -> Connection {
    return match Connection::open_with_flags(Path::new(DB_LOCATION), OpenFlags::default()) {
        Ok(con) => con,
        Err(error) => panic!("Failed to get a connection to the database!: {}", error),
    };
}

fn get_version(con: &mut Connection) -> Result<String> {
    let result = con.query_row(
        "select value from metadata where name = \"version\"",
        [],
        |row| row.get(0),
    );
    return result;
}

fn create_db(con: &mut Connection) {
    let sql = include_str!("../assets/init.sql");
    con.execute_batch(sql).unwrap();
}

pub fn initialize_db() -> Result<()> {
    let mut con = open_connection();
    let table_version = match get_version(&mut con) {
        Ok(value) => value.parse::<u64>().unwrap(),
        Err(_) => {
            // tables haven't been created yet
            create_db(&mut con);
            1
        }
    };
    // table_version will be used once we have more versions of the database
    match con.close() {
        Ok(_) => (),
        Err(e) => panic!("Failed to close connection!: {:?}", e),
    };
    Ok(())
}
