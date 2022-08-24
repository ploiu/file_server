use crate::db::folder_repository::get_by_id;
use crate::db::open_connection;
use crate::model::db::Folder;
use crate::service::folder_service::GetFolderError;

pub fn get_folder_by_id(id: u64) -> Result<Folder, GetFolderError> {
    let con = open_connection();
    let result = match get_by_id(id, &con) {
        Ok(folder) => Ok(folder),
        Err(error) if error == rusqlite::Error::QueryReturnedNoRows => {
            Err(GetFolderError::NotFound)
        }
        Err(err) => {
            eprintln!(
                "Failed to pull folder info from database! Nested exception is: \n {:?}",
                err
            );
            Err(GetFolderError::DbFailure)
        }
    };
    con.close().unwrap();
    result
}
