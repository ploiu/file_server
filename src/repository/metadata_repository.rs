use std::backtrace::Backtrace;

use rusqlite::Connection;

use crate::guard::HeaderAuth;
use crate::model::request::BodyAuth;
use crate::model::service::metadata::CheckAuthResult;

/// returns the current version of the database as a String
pub fn get_version(con: &Connection) -> Result<String, rusqlite::Error> {
    con.query_row(
        include_str!("../assets/queries/metadata/get_database_version.sql"),
        [],
        |row| row.get(0),
    )
}

/// retrieves the encrypted authentication string for requests in the database
pub fn get_auth(con: &Connection) -> Result<String, rusqlite::Error> {
    con.query_row(
        include_str!("../assets/queries/metadata/get_auth_hash.sql"),
        [],
        |row| row.get(0),
    )
}

/// checks if the passed `auth` matches the encrypted auth string in the database
pub fn check_auth(auth: HeaderAuth, con: &Connection) -> Result<CheckAuthResult, rusqlite::Error> {
    let hash = auth.to_string();
    match get_auth(con) {
        Ok(db_hash) => {
            if db_hash.eq(&hash) {
                Ok(CheckAuthResult::Valid)
            } else {
                Ok(CheckAuthResult::Invalid)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(CheckAuthResult::Missing),
        Err(e) => {
            log::error!(
                "Failed to check auth in database: {e:?}\n{}",
                Backtrace::force_capture()
            );
            Err(e)
        }
    }
}

pub fn set_auth(auth: HeaderAuth, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut statement = con
        .prepare(include_str!("../assets/queries/metadata/set_auth_hash.sql"))
        .unwrap();
    match statement.execute([auth.to_string()]) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!(
                "Failed to set password. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            Err(e)
        }
    }
}

pub fn update_auth(auth: BodyAuth, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut statement = con.prepare(include_str!(
        "../assets/queries/metadata/update_auth_hash.sql"
    ))?;
    statement.execute([auth.to_string()])?;
    Ok(())
}

pub fn get_generated_previews_flag(con: &Connection) -> Result<bool, rusqlite::Error> {
    let mut statement = con.prepare(include_str!(
        "../assets/queries/metadata/get_preview_generated_flag.sql"
    ))?;
    let query_res: Result<String, rusqlite::Error> = statement.query_row([], |it| it.get(0));
    if let Err(rusqlite::Error::QueryReturnedNoRows) = query_res {
        Ok(false)
    } else if let Err(e) = query_res {
        Err(e)
    } else {
        Ok(true)
    }
}

pub fn set_generated_previews_flag(con: &Connection) -> Result<(), rusqlite::Error> {
    let mut statement = con.prepare(include_str!(
        "../assets/queries/metadata/set_preview_generated_flag.sql"
    ))?;
    statement.execute([])?;
    Ok(())
}

pub fn get_generated_file_types_flag(con: &Connection) -> Result<bool, rusqlite::Error> {
    let mut check_flag_statement = con.prepare(include_str!(
        "../assets/queries/metadata/get_preview_generated_flag.sql"
    ))?;
    let query_res: Result<(), rusqlite::Error> = check_flag_statement.query_row([], |_| Ok(()));
    match query_res {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn set_generated_file_types_flag(con: &Connection) -> Result<(), rusqlite::Error> {
    let mut statement = con.prepare(include_str!(
        "../assets/queries/metadata/set_file_types_generated_flag.sql"
    ))?;
    statement.execute([])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{check_auth, set_auth, update_auth};
    use crate::guard::HeaderAuth;
    use crate::model::request::BodyAuth;
    use crate::model::service::metadata::CheckAuthResult;
    use crate::repository::open_connection;
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn update_auth_works() {
        refresh_db();
        let con: Connection = open_connection();
        set_auth(
            HeaderAuth {
                username: "username".to_string(),
                password: "password".to_string(),
            },
            &con,
        )
        .unwrap();
        update_auth(
            BodyAuth {
                username: "updated".to_string(),
                password: "updated".to_string(),
            },
            &con,
        )
        .unwrap();
        let res = check_auth(
            HeaderAuth {
                username: "updated".to_string(),
                password: "updated".to_string(),
            },
            &con,
        )
        .unwrap();
        con.close().unwrap();
        assert_eq!(CheckAuthResult::Valid, res);
        cleanup();
    }
}
