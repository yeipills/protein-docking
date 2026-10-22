/*!
 * Configuración de rutas de la API
 */

use actix_web::{web, HttpResponse};
use crate::api::handlers;

/// Configurar todas las rutas de la API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        // Health check
        .route("/health", web::get().to(handlers::health::health_check))
        .route("/metrics", web::get().to(handlers::health::metrics))
        
        // API v1
        .service(
            web::scope("/api/v1")
                .configure(configure_v1_routes)
        );
}

/// Configurar rutas API v1
fn configure_v1_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Protein docking endpoints
        .service(
            web::scope("/docking")
                .route("/upload", web::post().to(handlers::docking::upload_files))
                .route("/process", web::post().to(handlers::docking::process_docking))
                .route("/status/{job_id}", web::get().to(handlers::docking::get_status))
                .route("/results/{job_id}", web::get().to(handlers::docking::get_results))
                .route("/download/{job_id}", web::get().to(handlers::docking::download_results))
        )
        
        // File management
        .service(
            web::scope("/files")
                .route("/validate", web::post().to(handlers::files::validate_file))
                .route("/preview/{file_id}", web::get().to(handlers::files::preview_file))
        )
        
        // Algorithm management
        .service(
            web::scope("/algorithms")
                .route("/", web::get().to(handlers::algorithms::list_algorithms))
                .route("/benchmark", web::post().to(handlers::algorithms::run_benchmark))
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    
    #[actix_web::test]
    async fn test_health_endpoint() {
        let app = test::init_service(
            App::new().configure(configure)
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/health")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}