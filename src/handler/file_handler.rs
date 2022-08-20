use rocket::form::Form;
use rocket::http::Status;

use crate::guard::Auth;
use crate::model::request::FileUpload;
use crate::model::response::BasicResponse;
use crate::service::file_service::{save_file, FileError};

/// accepts a file via request body and stores it off
#[post("/", data = "<file_input>")]
pub async fn upload_file(
    file_input: Form<FileUpload<'_>>,
    auth: Auth,
) -> (Status, BasicResponse<'_>) {
    match auth.validate() {
        Some(v) => return v,
        _ => {}
    };
    return match save_file(&mut file_input.into_inner()).await {
        Ok(_) => BasicResponse::text(Status::NoContent, ""),
        Err(e) => match e {
            FileError::MissingInfo(messaeg) => BasicResponse::
            FileError::FailWriteDisk => {}
            FileError::FailWriteDb => {}
        }
    };
}
