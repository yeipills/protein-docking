/*!
 * Health check handlers
 */

use actix_web::{web, HttpResponse, Result};
use crate::core::{AppState, Result as AppResult};

/// Health check endpoint
pub async fn health_check(state: web::Data<AppState>) -> Result<HttpResponse> {
    let health_status = state.health_check().await;
    
    let status_code = match health_status.overall {
        crate::core::state::ComponentStatus::Healthy => 200,
        crate::core::state::ComponentStatus::Degraded => 503,
        crate::core::state::ComponentStatus::Unhealthy(_) => 503,
        crate::core::state::ComponentStatus::NotConfigured => 503,
    };
    
    Ok(HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status_code).unwrap()
    ).json(health_status))
}

/// System metrics endpoint
pub async fn metrics(state: web::Data<AppState>) -> Result<HttpResponse> {
    let metrics = state.system_metrics();
    Ok(HttpResponse::Ok().json(metrics))
}

/// Readiness probe for Kubernetes
pub async fn ready(state: web::Data<AppState>) -> Result<HttpResponse> {
    let health_status = state.health_check().await;
    
    match health_status.overall {
        crate::core::state::ComponentStatus::Healthy => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "ready",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        },
        _ => {
            Ok(HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "status": "not_ready",
                "reason": "System components not healthy",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        }
    }
}

/// Liveness probe for Kubernetes
pub async fn live() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    
    #[actix_web::test]
    async fn test_liveness_probe() {
        let resp = live().await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}