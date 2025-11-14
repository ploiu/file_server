use rusqlite::{Connection, Result};
use std::backtrace::Backtrace;

use crate::{
    repository::{file_repository, metadata_repository, open_connection},
    service::file_service,
};

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
        log_migration_version(2);
        migrate_v2(con)?;
    }
    if table_version < 3 {
        log_migration_version(3);
        migrate_v3(con)?;
    }
    if table_version < 4 {
        log_migration_version(4);
        migrate_v4(con)?;
    }
    if table_version < 5 {
        log_migration_version(5);
        migrate_v5(con)?;
    }
    if table_version < 6 {
        log_migration_version(6);
        migrate_v6(con)?;
    }
    Ok(())
}

fn log_migration_version(_version: u64) {
    #[cfg(not(test))]
    log::info!("Migrating database to v{_version}...");
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

fn migrate_v6(con: &Connection) -> Result<()> {
    con.execute_batch(include_str!("./assets/migration/v6.sql"))
}

#[cfg(test)]
mod migration_tests {
    use crate::repository::open_connection;
    use crate::test::{cleanup, init_db_folder};
    use rusqlite::params;

    #[test]
    fn v6_migration_adds_inherited_from_columns() {
        init_db_folder();
        let con = open_connection();
        
        // Create test data
        let tag_id = con.execute("insert into Tags(title) values (?1)", params!["test_tag"]).unwrap();
        let file_id = con.execute("insert into FileRecords(name) values (?1)", params!["test_file.txt"]).unwrap();
        let folder_id = con.execute("insert into Folders(name) values (?1)", params!["test_folder"]).unwrap();
        
        // Add tag to file and folder
        con.execute("insert into Files_Tags(fileRecordId, tagId) values (?1, ?2)", params![file_id, tag_id]).unwrap();
        con.execute("insert into Folders_Tags(folderId, tagId) values (?1, ?2)", params![folder_id, tag_id]).unwrap();
        
        // Verify inherited_from column exists and is NULL by default
        let file_inherited: Option<u32> = con.query_row(
            "select inherited_from from Files_Tags where fileRecordId = ?1",
            params![file_id],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(None, file_inherited);
        
        let folder_inherited: Option<u32> = con.query_row(
            "select inherited_from from Folders_Tags where folderId = ?1",
            params![folder_id],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(None, folder_inherited);
        
        // Verify we can set inherited_from
        let parent_folder_id = con.execute("insert into Folders(name) values (?1)", params!["parent_folder"]).unwrap();
        con.execute("update Files_Tags set inherited_from = ?1 where fileRecordId = ?2", params![parent_folder_id, file_id]).unwrap();
        con.execute("update Folders_Tags set inherited_from = ?1 where folderId = ?2", params![parent_folder_id, folder_id]).unwrap();
        
        let file_inherited: Option<u32> = con.query_row(
            "select inherited_from from Files_Tags where fileRecordId = ?1",
            params![file_id],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(Some(parent_folder_id as u32), file_inherited);
        
        let folder_inherited: Option<u32> = con.query_row(
            "select inherited_from from Folders_Tags where folderId = ?1",
            params![folder_id],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(Some(parent_folder_id as u32), folder_inherited);
        
        con.close().unwrap();
        cleanup();
    }
    
    #[test]
    fn v6_migration_preserves_unique_constraint() {
        init_db_folder();
        let con = open_connection();
        
        // Create test data
        con.execute("insert into Tags(title) values (?1)", params!["test_tag"]).unwrap();
        let file_id = con.execute("insert into FileRecords(name) values (?1)", params!["test_file.txt"]).unwrap();
        
        // Add tag to file
        con.execute("insert into Files_Tags(fileRecordId, tagId) values (?1, 1)", params![file_id]).unwrap();
        
        // Try to add the same tag again - should fail due to unique constraint
        let result = con.execute("insert into Files_Tags(fileRecordId, tagId) values (?1, 1)", params![file_id]);
        assert!(result.is_err(), "Expected unique constraint violation");
        
        con.close().unwrap();
        cleanup();
    }
}
