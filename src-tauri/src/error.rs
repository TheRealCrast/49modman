use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InternalError {
    #[error("{code}: {message}")]
    App {
        code: &'static str,
        message: String,
        detail: Option<String>,
    },
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    TimeParse(#[from] time::error::Parse),
    #[error(transparent)]
    TimeFormat(#[from] time::error::Format),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }
}

impl serde::Serialize for InternalError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_app_error().serialize(serializer)
    }
}

impl InternalError {
    pub fn app(code: &'static str, message: impl Into<String>) -> Self {
        Self::App {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(
        code: &'static str,
        message: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self::App {
            code,
            message: message.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn to_app_error(&self) -> AppError {
        match self {
            Self::App {
                code,
                message,
                detail,
            } => AppError {
                code,
                message: message.clone(),
                detail: detail.clone(),
            },
            Self::Sqlite(error) => AppError {
                code: "DB_INIT_FAILED",
                message: "SQLite operation failed".into(),
                detail: Some(error.to_string()),
            },
            Self::Json(error) => AppError {
                code: "THUNDERSTORE_RESPONSE_INVALID",
                message: "JSON serialization failed".into(),
                detail: Some(error.to_string()),
            },
            Self::Http(error) => AppError {
                code: "CATALOG_SYNC_FAILED",
                message: "HTTP request failed".into(),
                detail: Some(error.to_string()),
            },
            Self::TimeParse(error) => AppError {
                code: "CATALOG_SYNC_FAILED",
                message: "Timestamp parsing failed".into(),
                detail: Some(error.to_string()),
            },
            Self::TimeFormat(error) => AppError {
                code: "CATALOG_SYNC_FAILED",
                message: "Timestamp formatting failed".into(),
                detail: Some(error.to_string()),
            },
            Self::Io(error) => AppError {
                code: "RESOURCE_LOAD_FAILED",
                message: "Filesystem operation failed".into(),
                detail: Some(error.to_string()),
            },
        }
    }
}

impl From<InternalError> for AppError {
    fn from(value: InternalError) -> Self {
        value.to_app_error()
    }
}
