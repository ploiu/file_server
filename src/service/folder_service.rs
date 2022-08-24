use crate::facade::folder_facade::{get_child_folders_for, get_folder_by_id};
use crate::model::response::folder_responses::FolderResponse;

#[derive(PartialEq, Debug)]
pub enum GetFolderError {
    NotFound,
    DbFailure,
}

pub fn get_folder(id: u64) -> Result<FolderResponse, GetFolderError> {
    match get_folder_by_id(id) {
        Ok(folder) => {
            let mut folder = FolderResponse {
                // should always have an id when coming from the database
                id: folder.id.unwrap(),
                parent_id: folder.parent_id,
                path: folder.name,
                // TODO all nested folders
                folders: Vec::new(),
                // TODO all nested files
                files: Vec::new(),
            };
            folder.folders(get_child_folders_for(id).unwrap());
            Ok(folder)
        }
        Err(e) => Err(e),
    }
}
