use crate::facade::folder_facade::get_folder_by_id;
use crate::model::response::folder_responses::FolderResponse;

#[derive(PartialEq)]
pub enum GetFolderError {
    NotFound,
    DbFailure,
}

pub fn get_folder(id: u64) -> Result<FolderResponse, GetFolderError> {
    match get_folder_by_id(id) {
        Ok(folder) => Ok(FolderResponse {
            // should always have an id when coming from the database
            id: folder.id.unwrap(),
            path: folder.name,
            // TODO all nested folders
            folders: Vec::new(),
            // TODO all nested files
            files: Vec::new(),
        }),
        Err(e) => Err(e),
    }
}
