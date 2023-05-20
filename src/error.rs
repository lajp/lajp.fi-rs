use thiserror::Error;

use actix_web::{http::StatusCode, HttpResponse, ResponseError};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database error {0}")]
    Database(#[from] diesel::result::Error),
    #[error("Database Pool error {0}")]
    DbPool(#[from] r2d2::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).finish()
    }
}
