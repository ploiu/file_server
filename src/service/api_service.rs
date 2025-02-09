use rocket::serde::Serialize;
use rusqlite::Connection;
use std::backtrace::Backtrace;

use crate::guard::HeaderAuth;
use crate::model::error::metadata_errors::{CreatePasswordError, UpdatePasswordError};
use crate::model::request::{BodyAuth, UpdateAuth};
use crate::model::service::metadata::CheckAuthResult;
use crate::repository;
use crate::repository::{metadata_repository, open_connection};
use sysinfo::Disks;

use super::file_service::file_dir;
use std::path::Path;

/// reduced disk information retrieved from a [sysinfo::Disk]
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DiskInfo {
    /// the mounted folder name
    pub name: String,
    /// the total size in bytes the disk can hold
    #[serde(rename = "totalSpace")]
    pub total_space: u64,
    /// the written bytes to the disk
    #[serde(rename = "freeSpace")]
    pub free_space: u64,
}

pub enum DiskInfoError {
    Windows,
    Generic,
}

pub fn create_auth(auth: BodyAuth) -> Result<(), CreatePasswordError> {
    if is_password_set() {
        return Err(CreatePasswordError::AlreadyExists);
    }
    let auth = HeaderAuth {
        username: auth.username,
        password: auth.password,
    };
    if set_password(auth) {
        Ok(())
    } else {
        Err(CreatePasswordError::Failure)
    }
}

/// Checks if the passed `auth` object matches the password in the database
pub fn check_auth(auth: HeaderAuth) -> CheckAuthResult {
    let con = repository::open_connection();
    let result = metadata_repository::check_auth(auth, &con);
    con.close().unwrap();
    if let Ok(r) = result {
        r
    } else {
        CheckAuthResult::DbError
    }
}

pub fn update_auth(auth: UpdateAuth) -> Result<(), UpdatePasswordError> {
    log::info!("Attempting to update password...");
    let check_res = check_auth(auth.old_auth.into_auth());
    if check_res != CheckAuthResult::Valid {
        log::error!(
            "Failed to update authentication. Error is {check_res:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(UpdatePasswordError::Unauthorized);
    }
    // authorization matches, we can update
    let con: Connection = open_connection();
    if let Err(e) = metadata_repository::update_auth(auth.new_auth, &con) {
        log::error!(
            "Failed to update password! Error is {e}\n{}",
            Backtrace::force_capture()
        );
        con.close().unwrap();
        return Err(UpdatePasswordError::Unauthorized);
    }
    con.close().unwrap();
    Ok(())
}

/// retrieves the basic information for the disk our files folder is stored on (the result of [super::file_service::file_dir])
#[cfg(unix)]
pub fn get_disk_info() -> Result<DiskInfo, DiskInfoError> {
    let files_dir = match Path::new(&file_dir()).canonicalize() {
        Ok(path) => path,
        Err(e) => {
            log::error!(
                "Failed to get absolute path of file dir: {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(DiskInfoError::Generic);
        }
    };
    let device_id = get_device_id(&files_dir)?;
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        if device_id == get_device_id(&disk.mount_point())? {
            return Ok(DiskInfo {
                name: String::from(
                    disk.mount_point()
                        .to_str()
                        .or(Some("non-utf8 mount point path!"))
                        .unwrap(),
                ),
                total_space: disk.total_space(),
                free_space: disk.available_space(),
            });
        }
    }
    log::error!(
        "Failed to find which disk our files are stored on due to an unknown error\n{}",
        Backtrace::force_capture()
    );
    Err(DiskInfoError::Generic)
}

#[cfg(unix)]
fn get_device_id(path: &std::path::Path) -> Result<u64, DiskInfoError> {
    use std::os::unix::fs::MetadataExt;
    // instead of comparing path names, it's more accurate to compare device IDs (and is why I'm not supporting windows for this endpoint)
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.dev()),
        Err(e) => {
            log::error!(
                "Failed to get device id for {path:?}; {e:?}\n{}",
                Backtrace::force_capture()
            );
            return Err(DiskInfoError::Generic);
        }
    }
}

/// I don't _really_ care about windows, and using the windows bindings requires use of unsafe.
/// Because of the extra checks I'd need to do and the fact that I'm targeting a raspberry pi...this just returns Err
#[cfg(not(unix))]
pub fn get_disk_info() -> Result<!, DiskInfoError> {
    Err(DiskInfoError::Windows)
}

// private functions

/// checks if a password was set in the database
fn is_password_set() -> bool {
    let con = repository::open_connection();
    let auth_result = metadata_repository::get_auth(&con);
    con.close().unwrap();

    match auth_result {
        Ok(_) => true,
        Err(rusqlite::Error::QueryReturnedNoRows) => false,
        Err(e) => {
            panic!("Failed to check auth in database: {:?}", e);
        }
    }
}

/// saves the passed auth to the database.
///
/// This should never be called if there is a password already set (see `is_password_set`), because
/// it will override whatever is already in the database.
fn set_password(auth: HeaderAuth) -> bool {
    let con = repository::open_connection();
    let result = metadata_repository::set_auth(auth, &con);
    con.close().unwrap();
    result.is_ok()
}

#[cfg(test)]
mod tests {
    use crate::guard::HeaderAuth;
    use crate::model::error::metadata_errors::UpdatePasswordError;
    use crate::model::request::{BodyAuth, UpdateAuth};
    use crate::model::service::metadata::CheckAuthResult;
    use crate::service::api_service::{check_auth, create_auth, update_auth};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn update_auth_works() {
        refresh_db();
        create_auth(BodyAuth {
            username: "username".to_string(),
            password: "password".to_string(),
        })
        .unwrap();
        update_auth(UpdateAuth {
            old_auth: BodyAuth {
                username: "username".to_string(),
                password: "password".to_string(),
            },
            new_auth: BodyAuth {
                username: "updated".to_string(),
                password: "updated".to_string(),
            },
        })
        .unwrap();
        let res = check_auth(HeaderAuth {
            username: "updated".to_string(),
            password: "updated".to_string(),
        });
        assert_eq!(CheckAuthResult::Valid, res);
        cleanup();
    }

    #[test]
    fn update_auth_old_no_match() {
        refresh_db();
        create_auth(BodyAuth {
            username: "username".to_string(),
            password: "password".to_string(),
        })
        .unwrap();
        let res = update_auth(UpdateAuth {
            old_auth: BodyAuth {
                username: "UserName".to_string(),
                password: "password".to_string(),
            },
            new_auth: BodyAuth {
                username: "updated".to_string(),
                password: "updated".to_string(),
            },
        })
        .unwrap_err();
        assert_eq!(UpdatePasswordError::Unauthorized, res);
        cleanup();
    }

    #[test]
    fn update_auth_no_password_set() {
        refresh_db();
        let res = update_auth(UpdateAuth {
            old_auth: BodyAuth {
                username: "username".to_string(),
                password: "password".to_string(),
            },
            new_auth: BodyAuth {
                username: "updated".to_string(),
                password: "updated".to_string(),
            },
        })
        .unwrap_err();
        assert_eq!(UpdatePasswordError::Unauthorized, res);
        cleanup();
    }
}
