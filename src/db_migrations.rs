use std::path::Path;

#[cfg(not(test))]
use rusqlite::OpenFlags;
use rusqlite::{Connection, Result};
use std::backtrace::Backtrace;

use crate::{
    config::FILE_SERVER_CONFIG,
    queue,
    repository::{file_repository, metadata_repository, open_connection},
    service::file_service,
};

/// checks the database and generates previews for all files if the database doesn't have the flag `generated_previews` in the metadata table
#[deprecated]
pub fn generate_all_previews() {
    if !FILE_SERVER_CONFIG.clone().rabbit_mq.enabled {
        return;
    }
    log::info!("Starting to generate previews for existing files...");
    let con = open_connection();
    let flag_res = metadata_repository::get_generated_previews_flag(&con);
    if Ok(false) == flag_res {
        let file_ids = match file_repository::get_all_file_ids(&con) {
            Ok(ids) => ids,
            Err(e) => {
                con.close().unwrap();
                log::error!(
                    "Failed to retrieve all file IDs in the database. Error is {e:?}\n{}",
                    Backtrace::force_capture()
                );
                return;
            }
        };
        for id in file_ids {
            queue::publish_message("icon_gen", &id.to_string());
        }
        let flag_set_result = metadata_repository::set_generated_previews_flag(&con);
        con.close().unwrap();
        if let Err(e) = flag_set_result {
            log::error!(
                "Failed to set preview flag in database. Exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
        } else {
            log::info!("Successfully pushed file IDs to queue")
        }
    } else if let Err(e) = flag_res {
        log::error!(
            "Failed to get preview flag from database. Error is {e:?}\n{}",
            Backtrace::force_capture()
        );
        con.close().unwrap();
        return;
    } else {
        log::info!("Not generating file previews because the db flag is already set.")
    }
}

/// pulls all pre-existing files in the database and updates the `type` column to be the correct [crate::model::file_types::FileTypes]
/// This function only performs the update if the flag controlling this has not been set in the metadata table
pub fn generate_all_file_types_and_sizes() {
    log::info!("Starting to generate file types and sizes for all existing files...");
    let con = open_connection();
    let flag = metadata_repository::get_generated_file_types_flag(&con);
    if let Err(e) = flag {
        con.close().unwrap();
        log::error!(
            "Failed to check database: {e:?}\n{}",
            Backtrace::force_capture()
        );
        return;
    }
    let flag = flag.unwrap();
    if !flag {
        let files = match file_repository::get_all_files(&con) {
            Ok(f) => f,
            Err(e) => {
                log::error!(
                    "Failed to retrieve all files from the database: {e:?}\n{}",
                    Backtrace::force_capture()
                );
                con.close().unwrap();
                return;
            }
        };
        let mut sql = String::from("Begin;\n");
        for file in files {
            let id = file.id.unwrap();
            let file_type = file_service::determine_file_type(&file.name).to_string();
            let path = match file_repository::get_file_path(id, &con) {
                Ok(p) => p,
                Err(e) => {
                    con.close().unwrap();
                    log::error!(
                        "Failed to determine file size for {}: {e:?}\n{}",
                        file.name,
                        Backtrace::force_capture()
                    );
                    return;
                }
            };
            let size = match std::fs::metadata(format!("./files/{}", path.clone())) {
                Ok(metadata) => metadata.len(),
                Err(e) => {
                    con.close().unwrap();
                    log::error!(
                        "Failed to get metadata for {path}; {e:?}\n{}",
                        Backtrace::force_capture()
                    );
                    return;
                }
            };
            sql += format!(
                r"update FileRecords 
                set type = '{file_type}', fileSize = {size}
                where id = {id};
                ",
            )
            .as_str();
        }
        sql += "commit;";
        let res = con.execute_batch(&sql);
        if res.is_ok() {
            let flag_res = metadata_repository::set_generated_file_types_flag(&con);
            con.close().unwrap();
            if let Err(e) = flag_res {
                log::error!(
                    "Failed to set the db flag for file types: {e:?}\n{}",
                    Backtrace::force_capture()
                );
            } else {
                log::info!("Successfully finished populating pre-existing file types!");
            }
        } else {
            con.close().unwrap();
            log::error!(
                "Failed to batch update file types and sizes in database: {:?}\n{}",
                res.unwrap_err(),
                Backtrace::force_capture()
            );
        }
    } else {
        log::info!("Not generating file types and sizes because flag is already set...");
    }
}

/// incrementally upgrades the database for each version the database is behind
pub fn migrate_db(con: &Connection, table_version: u64) -> Result<()> {
    if table_version < 2 {
        log::info!("Migrating database to v2...");
        migrate_v2(con)?;
    }
    if table_version < 3 {
        log::info!("Migrating database to v3...");
        migrate_v3(con)?;
    }
    if table_version < 4 {
        log::info!("Migrating database to v4...");
        migrate_v4(con)?;
    }
    if table_version < 5 {
        log::info!("Migrating database to v5...");
        migrate_v5(con)?;
    }
    Ok(())
}

fn migrate_v2(con: &Connection) -> Result<()> {
    let migration_script = include_str!("./assets/migration/v2.sql");
    con.execute_batch(migration_script)
}

fn migrate_v3(con: &Connection) -> Result<()> {
    let migration_script = include_str!("./assets/migration/v3.sql");
    con.execute_batch(migration_script)
}

fn migrate_v4(con: &Connection) -> Result<()> {
    let migration_script = include_str!("./assets/migration/v4.sql");
    con.execute_batch(migration_script)
}

fn migrate_v5(con: &Connection) -> Result<()> {
    con.execute_batch(include_str!("./assets/migration/v5.sql"))
}
