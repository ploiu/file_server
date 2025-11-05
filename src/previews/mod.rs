pub mod handler;
pub mod models;
mod service;

pub use service::{
    delete_file_preview, generate_preview, get_file_preview,
    load_all_files_in_preview_queue,
};

#[cfg(not(test))]
fn preview_dir() -> String {
    "./file_previews".to_string()
}

#[cfg(test)]
pub fn preview_dir() -> String {
    let thread_name = crate::test::current_thread_name();
    let dir_name = format!("./{thread_name}_previews");
    dir_name
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub use service::ensure_preview_dir;
