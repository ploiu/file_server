use chrono::NaiveDateTime;
use rusqlite::Connection;
use std::backtrace::Backtrace;

/// Updates the creation date of a file in the database.
///
/// ## Parameters
/// * `file_id` - The ID of the file to update
/// * `create_date` - The new creation date to set
/// * `con` - The database connection
///
/// ## Returns
/// * `Ok(())` if the update was successful
/// * `Err(rusqlite::Error)` if the update failed
pub fn update_file_create_date(
    file_id: u32,
    create_date: NaiveDateTime,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let update_result = con.execute(
        "UPDATE FileRecords SET dateCreated = ?1 WHERE id = ?2",
        rusqlite::params![create_date, file_id],
    );

    match update_result {
        Ok(_) => {
            log::debug!("Successfully updated creation date for file id {file_id}");
            Ok(())
        }
        Err(e) => {
            log::error!(
                "Failed to update creation date for file id {file_id}: {e:?}\n{}",
                Backtrace::force_capture()
            );
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::file_types::FileTypes;
    use crate::model::repository::FileRecord;
    use crate::repository::open_connection;
    use crate::test::{cleanup, init_db_folder};

    #[test]
    fn update_file_create_date_updates_date() {
        init_db_folder();
        
        // Create a file with an old date
        let old_date = NaiveDateTime::parse_from_str("2000-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let file_record = FileRecord {
            id: None,
            name: "test.png".to_string(),
            parent_id: None,
            create_date: old_date,
            size: 100,
            file_type: FileTypes::Image,
        }.save_to_db();

        let file_id = file_record.id.unwrap();

        // Update the date
        let new_date = NaiveDateTime::parse_from_str("2024-01-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let con = open_connection();
        let result = update_file_create_date(file_id, new_date, &con);
        con.close().unwrap();

        assert!(result.is_ok(), "Should successfully update the date");

        // Verify the date was updated
        let con = open_connection();
        let updated_record = crate::repository::file_repository::get_file(file_id, &con).unwrap();
        con.close().unwrap();

        assert_eq!(updated_record.create_date, new_date, "Date should be updated");
        
        cleanup();
    }

    #[test]
    fn update_file_create_date_fails_for_nonexistent_file() {
        init_db_folder();
        
        let new_date = NaiveDateTime::parse_from_str("2024-01-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let con = open_connection();
        let result = update_file_create_date(999, new_date, &con);
        con.close().unwrap();

        // Should succeed but not update any rows (SQLite doesn't error for 0 rows updated)
        assert!(result.is_ok());
        
        cleanup();
    }
}
