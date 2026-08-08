use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub details: Option<String>,
}

impl ApiError {
    pub fn new(error: impl Into<String>, details: Option<String>) -> Self {
        Self {
            error: error.into(),
            details,
        }
    }

    pub fn bad_request(error: impl Into<String>) -> Self {
        Self::new(error, None)
    }

    pub fn unauthorized(error: impl Into<String>) -> Self {
        Self::new(error, None)
    }

    pub fn forbidden(error: impl Into<String>) -> Self {
        Self::new(error, None)
    }

    pub fn not_found(error: impl Into<String>) -> Self {
        Self::new(error, None)
    }

    pub fn conflict(error: impl Into<String>) -> Self {
        Self::new(error, None)
    }

    pub fn internal_error(error: impl Into<String>) -> Self {
        Self::new(error, None)
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

// Implement Writer for ApiError - match the exact signature
#[async_trait]
impl Writer for ApiError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        let status_code = match self.error.as_str() {
            "Bad Request" => StatusCode::BAD_REQUEST,
            "Unauthorized" => StatusCode::UNAUTHORIZED,
            "Forbidden" => StatusCode::FORBIDDEN,
            "Not Found" => StatusCode::NOT_FOUND,
            "Conflict" => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        res.status_code(status_code);
        res.render(Json(self));
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for ApiError {}

impl From<sea_orm::DbErr> for ApiError {
    fn from(err: sea_orm::DbErr) -> Self {
        match err {
            sea_orm::DbErr::RecordNotFound(_) => ApiError::not_found("Record not found"),
            sea_orm::DbErr::RecordNotInserted => ApiError::conflict("Record already exists"),
            _ => ApiError::internal_error("Database error").with_details(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::bad_request("Invalid JSON").with_details(err.to_string())
    }
}
