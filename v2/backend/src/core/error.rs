/*!
 * Sistema de manejo de errores centralizado
 */

use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum AppError {
    #[error("Internal server error: {message}")]
    Internal { message: String },
    
    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },
    
    #[error("File processing error: {message}")]
    FileProcessing { message: String },
    
    #[error("Algorithm computation error: {message}")]
    Algorithm { message: String },
    
    #[error("Database error: {message}")]
    Database { message: String },
    
    #[error("Cache error: {message}")]
    Cache { message: String },
    
    #[error("GPU computation error: {message}")]
    Gpu { message: String },
    
    #[error("Memory allocation error: {message}")]
    Memory { message: String },
    
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("Authentication error: {message}")]
    Auth { message: String },
    
    #[error("Authorization error: {message}")]
    AuthZ { message: String },
    
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },
    
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },
    
    #[error("Conflict: {message}")]
    Conflict { message: String },
    
    #[error("Timeout: {operation} took too long")]
    Timeout { operation: String },
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: u16,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl AppError {
    pub fn internal<T: fmt::Display>(msg: T) -> Self {
        Self::Internal {
            message: msg.to_string(),
        }
    }
    
    pub fn validation<T: fmt::Display>(field: &str, msg: T) -> Self {
        Self::Validation {
            field: field.to_string(),
            message: msg.to_string(),
        }
    }
    
    pub fn file_processing<T: fmt::Display>(msg: T) -> Self {
        Self::FileProcessing {
            message: msg.to_string(),
        }
    }
    
    pub fn algorithm<T: fmt::Display>(msg: T) -> Self {
        Self::Algorithm {
            message: msg.to_string(),
        }
    }
    
    pub fn database<T: fmt::Display>(msg: T) -> Self {
        Self::Database {
            message: msg.to_string(),
        }
    }
    
    pub fn gpu<T: fmt::Display>(msg: T) -> Self {
        Self::Gpu {
            message: msg.to_string(),
        }
    }
    
    pub fn not_found<T: fmt::Display>(resource: T) -> Self {
        Self::NotFound {
            resource: resource.to_string(),
        }
    }
    
    pub fn timeout<T: fmt::Display>(operation: T) -> Self {
        Self::Timeout {
            operation: operation.to_string(),
        }
    }
    
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::Internal { .. } => 500,
            AppError::Validation { .. } => 400,
            AppError::FileProcessing { .. } => 400,
            AppError::Algorithm { .. } => 500,
            AppError::Database { .. } => 500,
            AppError::Cache { .. } => 500,
            AppError::Gpu { .. } => 500,
            AppError::Memory { .. } => 500,
            AppError::Network { .. } => 502,
            AppError::Auth { .. } => 401,
            AppError::AuthZ { .. } => 403,
            AppError::RateLimit { .. } => 429,
            AppError::NotFound { .. } => 404,
            AppError::Conflict { .. } => 409,
            AppError::Timeout { .. } => 408,
        }
    }
    
    pub fn error_type(&self) -> &'static str {
        match self {
            AppError::Internal { .. } => "INTERNAL_ERROR",
            AppError::Validation { .. } => "VALIDATION_ERROR",
            AppError::FileProcessing { .. } => "FILE_PROCESSING_ERROR",
            AppError::Algorithm { .. } => "ALGORITHM_ERROR",
            AppError::Database { .. } => "DATABASE_ERROR",
            AppError::Cache { .. } => "CACHE_ERROR",
            AppError::Gpu { .. } => "GPU_ERROR",
            AppError::Memory { .. } => "MEMORY_ERROR",
            AppError::Network { .. } => "NETWORK_ERROR",
            AppError::Auth { .. } => "AUTHENTICATION_ERROR",
            AppError::AuthZ { .. } => "AUTHORIZATION_ERROR",
            AppError::RateLimit { .. } => "RATE_LIMIT_ERROR",
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Conflict { .. } => "CONFLICT",
            AppError::Timeout { .. } => "TIMEOUT",
        }
    }
    
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.error_type().to_string(),
            message: self.to_string(),
            code: self.status_code(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            details: None,
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let error_response = self.to_error_response();
        let status_code = actix_web::http::StatusCode::from_u16(self.status_code())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
        
        HttpResponse::build(status_code).json(error_response)
    }
}

// Conversiones desde errores comunes
// NOTA: sqlx está deshabilitado temporalmente
// impl From<sqlx::Error> for AppError {
//     fn from(err: sqlx::Error) -> Self {
//         AppError::Database {
//             message: err.to_string(),
//         }
//     }
// }

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Cache {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::FileProcessing {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Validation {
            field: "json".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<tokio::time::error::Elapsed> for AppError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        AppError::Timeout {
            operation: err.to_string(),
        }
    }
}

// Macro para crear errores rápidamente
#[macro_export]
macro_rules! app_error {
    (internal: $msg:expr) => {
        $crate::core::error::AppError::internal($msg)
    };
    (validation: $field:expr, $msg:expr) => {
        $crate::core::error::AppError::validation($field, $msg)
    };
    (not_found: $resource:expr) => {
        $crate::core::error::AppError::not_found($resource)
    };
    (algorithm: $msg:expr) => {
        $crate::core::error::AppError::algorithm($msg)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_status_codes() {
        assert_eq!(AppError::internal("test").status_code(), 500);
        assert_eq!(AppError::validation("field", "test").status_code(), 400);
        assert_eq!(AppError::not_found("resource").status_code(), 404);
    }
    
    #[test]
    fn test_error_response_serialization() {
        let error = AppError::validation("email", "Invalid format");
        let response = error.to_error_response();
        
        assert_eq!(response.error, "VALIDATION_ERROR");
        assert_eq!(response.code, 400);
        assert!(response.message.contains("Invalid format"));
    }
}