use rocket::http::{ContentType, Status};

#[derive(Responder)]
pub struct BasicResponse<'a> {
    body: &'a str,
    content_type: ContentType,
}

impl BasicResponse<'_> {
    pub fn text(status: Status, body: &str) -> (Status, BasicResponse) {
        (
            status,
            BasicResponse {
                content_type: ContentType::Text,
                body,
            },
        )
    }

    pub fn json(status: Status, body: &str) -> (Status, BasicResponse) {
        (
            status,
            BasicResponse {
                content_type: ContentType::JSON,
                body,
            },
        )
    }
}
