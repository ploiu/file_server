pub mod service;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub use service::process_single_file_exif;

use crate::repository::{metadata_repository, open_connection};

pub fn load_all_exif_data() {
    let con = open_connection();
    let exif_flag = metadata_repository::get_exif_processed_flag(&con);
    if let Ok(false) = exif_flag {
        log::info!("EXIF processing flag not set, queuing all image/video files for processing");
        service::mass_exif_process();
        let _ = metadata_repository::set_exif_processed_flag(&con);
    }
    con.close().unwrap();
}
