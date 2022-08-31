use rusqlite::Connection;

use crate::db::file_repository::delete_by_id;
use crate::model::db::FileRecord;
use crate::service::file_service::DeleteFileError;

/// for use in folder_facade, so that way we don't create a new connection every time
pub fn delete_file_by_id_with_connection(
    id: u32,
    con: &Connection,
) -> Result<FileRecord, DeleteFileError> {
    let result = match delete_by_id(id, &con) {
        Ok(record) => Ok(record),
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Err(DeleteFileError::NotFound),
        Err(e) => {
            eprintln!(
                "Failed to delete file record from database! Nested exception is: \n {:?}",
                e
            );
            Err(DeleteFileError::DbError)
        }
    };
    return result;
}
