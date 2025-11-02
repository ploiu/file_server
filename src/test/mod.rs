pub mod api_handler_tests;
pub mod file_handler_tests;
pub mod folder_handler_tests;

#[cfg(test)]
pub use tests::*;

#[cfg(test)]
mod tests {
    use crate::model::api::FileApi;
    use crate::model::repository::{FileRecord, Folder, Tag};
    use crate::previews;
    use crate::repository::{
        file_repository, folder_repository, initialize_db, open_connection, tag_repository,
    };
    use crate::service::file_service::{determine_file_type, file_dir};
    use crate::temp_dir;
    use std::fs;
    use std::fs::{remove_dir_all, remove_file};
    use std::path::Path;

    /// username:password
    pub static AUTH: &str = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";

    pub fn init_db_folder() {
        // since this is just for testing, we don't need to unwrap the logging
        let _ = fern::Dispatch::new()
            .level(log::LevelFilter::Debug)
            .level_for("rocket", log::LevelFilter::Off)
            .level_for("file_server::db_migrations", log::LevelFilter::Off)
            .chain(std::io::stdout())
            .apply();
        let thread_name = current_thread_name();
        fs::create_dir_all(file_dir()).expect("Failed to create base file dir");
        remove_file(Path::new(format!("{thread_name}.sqlite").as_str())).unwrap_or(());
        initialize_db().unwrap();
    }

    pub fn remove_files() {
        let thread_name = current_thread_name();
        let file_path = Path::new(thread_name.as_str());
        if file_path.exists() {
            remove_dir_all(file_path).unwrap_or(());
        }
    }

    pub fn remove_previews() {
        let preview_dir = previews::preview_dir();
        let preview_path = Path::new(preview_dir.as_str());
        if preview_path.exists() {
            remove_dir_all(preview_path).unwrap_or(());
        }
    }

    /// for quick and easy storing of a basic file in the database without any extra fields
    /// see also [`FileRecord::save_to_db`]
    pub fn create_file_db_entry(name: &str, folder_id: Option<u32>) {
        let file_type = determine_file_type(name);

        let connection = open_connection();
        let file_id = file_repository::create_file(
            &FileRecord {
                id: None,
                name: String::from(name),
                parent_id: folder_id,
                size: 0,
                create_date: now(),
                file_type,
            },
            &connection,
        )
        .unwrap();
        if let Some(id) = folder_id {
            folder_repository::link_folder_to_file(file_id, id, &connection).unwrap();
        }
        connection.close().unwrap();
    }

    pub fn create_file_preview(file_id: u32) {
        previews::ensure_preview_dir();
        let full_path = format!("{}/{}.png", previews::preview_dir(), file_id);
        fs::File::create(full_path).unwrap();
    }

    pub fn create_folder_db_entry(name: &str, parent_id: Option<u32>) {
        let connection = open_connection();
        folder_repository::create_folder(
            &Folder {
                id: None,
                name: String::from(name),
                parent_id,
            },
            &connection,
        )
        .unwrap();
        connection.close().unwrap();
    }

    pub fn create_tag_db_entry(name: &str) -> u32 {
        let connection = open_connection();
        let id = tag_repository::create_tag(name, &connection).unwrap().id;
        connection.close().unwrap();
        id
    }

    pub fn create_tag_folder(name: &str, folder_id: u32) {
        let connection = open_connection();
        let id = create_tag_db_entry(name);
        tag_repository::add_tag_to_folder(folder_id, id, &connection).unwrap();
        connection.close().unwrap();
    }

    pub fn create_tag_folders(name: &str, folder_ids: Vec<u32>) {
        let connection = open_connection();
        let id = create_tag_db_entry(name);
        for folder_id in folder_ids {
            tag_repository::add_tag_to_folder(folder_id, id, &connection).unwrap();
        }
        connection.close().unwrap();
    }

    pub fn create_tag_file(name: &str, file_id: u32) {
        let connection = open_connection();
        let id = create_tag_db_entry(name);
        tag_repository::add_tag_to_file(file_id, id, &connection).unwrap();
        connection.close().unwrap();
    }

    pub fn create_tag_files(name: &str, file_ids: Vec<u32>) {
        let connection = open_connection();
        let id = create_tag_db_entry(name);
        for file_id in file_ids {
            tag_repository::add_tag_to_file(file_id, id, &connection).unwrap();
        }
        connection.close().unwrap();
    }

    /// fails a test. macro instead of a function so that the stack shows the line in the test instead of where this is declared
    #[macro_export]
    macro_rules! fail {
        () => {
            panic!("unimplemented test")
        };
        ($msg:literal) => {
            panic!($msg)
        };
    }

    #[cfg(not(target_family = "windows"))]
    pub fn current_thread_name() -> String {
        let current_thread = std::thread::current();
        current_thread.name().unwrap().to_string()
    }

    #[cfg(target_family = "windows")]
    pub fn current_thread_name() -> String {
        let current_thread = std::thread::current();
        let current_thread_name = current_thread.name().unwrap().to_string();
        current_thread_name.replace(":", "_")
    }

    pub fn create_file_disk(file_name: &str, contents: &str) {
        fs::create_dir(Path::new(file_dir().as_str())).unwrap_or(());
        fs::write(
            Path::new(format!("{}/{file_name}", file_dir()).as_str()),
            contents,
        )
        .unwrap();
    }

    pub fn create_folder_disk(folder_name: &str) {
        fs::create_dir_all(Path::new(format!("{}/{folder_name}", file_dir()).as_str())).unwrap();
    }

    pub fn cleanup() {
        let thread_name = current_thread_name();
        let temp_dir_name = temp_dir();
        remove_files();
        remove_previews();
        remove_file(Path::new(format!("{thread_name}.sqlite").as_str())).unwrap_or(());
        remove_dir_all(Path::new(temp_dir_name.as_str())).unwrap_or(());
    }

    pub fn now() -> chrono::NaiveDateTime {
        chrono::offset::Local::now().naive_local()
    }

    // these partialEq implementations are because NaiveDate generation is too inconsistent to test around, so these test implementations do not test the date
    #[allow(clippy::derived_hash_with_manual_eq)]
    impl PartialEq for FileRecord {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
                && self.name == other.name
                && self.parent_id == other.parent_id
                && self.size == other.size
        }
    }

    impl PartialEq for FileApi {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
                && self.folder_id == other.folder_id
                && self.name == other.name
                && self.tags == other.tags
                && self.size == other.size
                && self.file_type == other.file_type
        }
    }

    impl FileApi {
        pub fn save_to_db(mut self) -> Self {
            let con = open_connection();
            let record = FileRecord {
                id: None,
                name: self.name.clone(),
                parent_id: self.folder_id,
                create_date: self.date_created.unwrap_or_default(),
                size: self.size.unwrap_or_default(),
                file_type: self.file_type.unwrap_or_default(),
            };
            let file_id = file_repository::create_file(&record, &con).unwrap();
            for tag in &mut self.tags {
                let Tag { id, title: _ } = tag_repository::create_tag(&tag.title, &con).unwrap();
                tag_repository::add_tag_to_file(file_id, id, &con).unwrap();
                tag.id = Some(id);
            }
            if let Some(folder_id) = self.folder_id {
                folder_repository::link_folder_to_file(file_id, folder_id, &con).unwrap();
            }
            con.close().unwrap();
            Self {
                id: file_id,
                folder_id: self.folder_id,
                name: self.name.clone(),
                tags: self.tags.clone(),
                size: self.size,
                date_created: self.date_created,
                file_type: self.file_type,
            }
        }
    }

    impl FileRecord {
        /// to be used when [`create_file_db_entry`] doesn't work for all the fields being set. This gives more granular control.
        pub fn save_to_db(self) -> Self {
            let con = open_connection();
            let file_id = file_repository::create_file(&self, &con).unwrap();
            if let Some(id) = self.parent_id {
                folder_repository::link_folder_to_file(file_id, id, &con).unwrap();
            }
            con.close().unwrap();
            Self {
                id: Some(file_id),
                name: self.name.clone(),
                parent_id: self.parent_id,
                create_date: self.create_date,
                size: self.size,
                file_type: self.file_type,
            }
        }
    }
}
