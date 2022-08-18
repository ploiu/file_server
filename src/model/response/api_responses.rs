type NoContent = ();

#[derive(Responder)]
pub enum SetPassWordResponse {
    #[response(status = 201)]
    Created(NoContent),
    #[response(status = 500, content_type = "json")]
    Failure(String),
}
