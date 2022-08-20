use std::fs::File;

type NoContent = ();

#[derive(Responder)]
pub enum GetFileResponse {
    #[response(status = 404, content_type = "json")]
    FileNotFound(String),
    #[response(status = 500, content_type = "json")]
    FileDbError(String),
    #[response(status = 200)]
    Success(File),
    #[response(status = 401)]
    Unauthorized(String),
}

#[derive(Responder)]
pub enum CreateFileResponse {
    #[response(status = 201)]
    Created(NoContent),
    #[response(status = 400, content_type = "json")]
    BadRequest(String),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 500, content_type = "json")]
    Failure(String),
}
