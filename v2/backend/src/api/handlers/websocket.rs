// WebSocket Handler - Comunicación en tiempo real para jobs de docking
// Manejo de conexiones WebSocket, broadcasting de progreso

use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_ws::{AggregatedMessage, Session};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use futures_util::{SinkExt, StreamExt};

use super::jobs::{JobManager, JobProgressUpdate};

/// Mensaje WebSocket para jobs
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    /// Cliente se subscribe a updates de un job
    Subscribe { job_id: String },
    /// Cliente se unsubscribe de un job
    Unsubscribe { job_id: String },
    /// Ping para mantener conexión viva
    Ping,
    /// Pong response
    Pong,
    /// Update de progreso de job
    JobProgress(JobProgressUpdate),
    /// Error en el WebSocket
    Error { message: String },
    /// Confirmación de subscription
    Subscribed { job_id: String },
    /// Confirmación de unsubscription
    Unsubscribed { job_id: String },
}

/// Contexto de conexión WebSocket
struct WebSocketContext {
    job_manager: Arc<JobManager>,
    session: Session,
    subscribed_jobs: Vec<String>,
}

/// Handler principal para conexiones WebSocket de jobs
pub async fn websocket_jobs_handler(
    req: HttpRequest,
    stream: web::Payload,
    job_manager: web::Data<Arc<JobManager>>,
) -> Result<HttpResponse> {
    let (res, session, stream) = actix_ws::handle(&req, stream)?;
    
    info!("🔌 New WebSocket connection established");
    
    // Crear contexto de conexión
    let mut ctx = WebSocketContext {
        job_manager: job_manager.get_ref().clone(),
        session,
        subscribed_jobs: Vec::new(),
    };
    
    // Manejar conexión en tarea separada
    actix_web::rt::spawn(async move {
        if let Err(e) = handle_websocket_connection(&mut ctx, stream).await {
            error!("WebSocket connection error: {}", e);
        }
        
        // Cleanup al cerrar conexión
        cleanup_websocket_connection(&ctx).await;
        info!("🔌 WebSocket connection closed");
    });
    
    Ok(res)
}

/// Manejar mensajes de la conexión WebSocket
async fn handle_websocket_connection(
    ctx: &mut WebSocketContext,
    mut stream: actix_ws::MessageStream,
) -> anyhow::Result<()> {
    // Canal para recibir updates de jobs
    let (tx, mut rx) = mpsc::unbounded_channel::<JobProgressUpdate>();
    
    // Enviar mensaje de bienvenida
    send_websocket_message(
        &mut ctx.session,
        &WebSocketMessage::Pong,
    ).await?;
    
    loop {
        tokio::select! {
            // Mensaje del cliente
            msg = stream.next() => {
                match msg {
                    Some(Ok(AggregatedMessage::Text(text))) => {
                        if let Err(e) = handle_client_message(ctx, &text, &tx).await {
                            warn!("Error handling client message: {}", e);
                            send_websocket_message(
                                &mut ctx.session,
                                &WebSocketMessage::Error {
                                    message: format!("Invalid message: {}", e)
                                }
                            ).await?;
                        }
                    },
                    Some(Ok(AggregatedMessage::Binary(_))) => {
                        warn!("Binary messages not supported");
                    },
                    Some(Ok(AggregatedMessage::Ping(payload))) => {
                        debug!("Received ping");
                        ctx.session.pong(&payload).await?;
                    },
                    Some(Ok(AggregatedMessage::Pong(_))) => {
                        debug!("Received pong");
                    },
                    Some(Ok(AggregatedMessage::Close(reason))) => {
                        info!("WebSocket closed: {:?}", reason);
                        break;
                    },
                    Some(Err(e)) => {
                        error!("WebSocket message error: {}", e);
                        break;
                    },
                    None => {
                        debug!("WebSocket stream ended");
                        break;
                    }
                }
            },
            
            // Update de progreso de job
            update = rx.recv() => {
                if let Some(update) = update {
                    send_websocket_message(
                        &mut ctx.session,
                        &WebSocketMessage::JobProgress(update)
                    ).await?;
                } else {
                    debug!("Job update channel closed");
                    break;
                }
            }
        }
    }
    
    Ok(())
}

/// Manejar mensaje del cliente
async fn handle_client_message(
    ctx: &mut WebSocketContext,
    text: &str,
    progress_tx: &mpsc::UnboundedSender<JobProgressUpdate>,
) -> anyhow::Result<()> {
    let message: WebSocketMessage = serde_json::from_str(text)?;
    
    match message {
        WebSocketMessage::Subscribe { job_id } => {
            info!("📋 Client subscribing to job: {}", job_id);
            
            // Verificar que el job existe
            if ctx.job_manager.get_job(&job_id).await.is_some() {
                // Registrar en job manager
                ctx.job_manager.register_websocket(&job_id, progress_tx.clone()).await;
                
                // Añadir a lista local
                if !ctx.subscribed_jobs.contains(&job_id) {
                    ctx.subscribed_jobs.push(job_id.clone());
                }
                
                // Confirmar subscription
                send_websocket_message(
                    &mut ctx.session,
                    &WebSocketMessage::Subscribed { job_id: job_id.clone() }
                ).await?;
                
                // Enviar estado actual del job si existe
                if let Some(job) = ctx.job_manager.get_job(&job_id).await {
                    let update = JobProgressUpdate {
                        job_id: job.job_id,
                        status: job.status,
                        progress_percent: job.progress_percent,
                        current_step: job.current_step,
                        poses_evaluated: job.computation_stats.as_ref()
                            .map(|s| s.total_poses_evaluated)
                            .unwrap_or(0),
                        poses_remaining: job.config.max_poses.saturating_sub(
                            job.computation_stats.as_ref()
                                .map(|s| s.total_poses_evaluated)
                                .unwrap_or(0)
                        ),
                        estimated_completion: job.estimated_completion,
                        computation_stats: job.computation_stats,
                        timestamp: chrono::Utc::now(),
                    };
                    
                    send_websocket_message(
                        &mut ctx.session,
                        &WebSocketMessage::JobProgress(update)
                    ).await?;
                }
            } else {
                send_websocket_message(
                    &mut ctx.session,
                    &WebSocketMessage::Error {
                        message: format!("Job not found: {}", job_id)
                    }
                ).await?;
            }
        },
        
        WebSocketMessage::Unsubscribe { job_id } => {
            info!("📋 Client unsubscribing from job: {}", job_id);
            
            // Desregistrar del job manager
            ctx.job_manager.unregister_websocket(&job_id).await;
            
            // Remover de lista local
            ctx.subscribed_jobs.retain(|id| id != &job_id);
            
            // Confirmar unsubscription
            send_websocket_message(
                &mut ctx.session,
                &WebSocketMessage::Unsubscribed { job_id }
            ).await?;
        },
        
        WebSocketMessage::Ping => {
            debug!("Received ping from client");
            send_websocket_message(
                &mut ctx.session,
                &WebSocketMessage::Pong
            ).await?;
        },
        
        _ => {
            warn!("Unsupported message type from client");
            send_websocket_message(
                &mut ctx.session,
                &WebSocketMessage::Error {
                    message: "Unsupported message type".to_string()
                }
            ).await?;
        }
    }
    
    Ok(())
}

/// Enviar mensaje via WebSocket
async fn send_websocket_message(
    session: &mut actix_ws::Session,
    message: &WebSocketMessage,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(message)?;
    session.text(json).await?;
    Ok(())
}

/// Cleanup al cerrar conexión
async fn cleanup_websocket_connection(ctx: &WebSocketContext) {
    // Desregistrar todas las subscriptions
    for job_id in &ctx.subscribed_jobs {
        ctx.job_manager.unregister_websocket(job_id).await;
    }
    
    info!("🧹 Cleaned up {} job subscriptions", ctx.subscribed_jobs.len());
}

/// Handler específico para WebSocket de un job individual
pub async fn websocket_job_handler(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
    job_manager: web::Data<Arc<JobManager>>,
) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    
    // Verificar que el job existe
    if job_manager.get_job(&job_id).await.is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Job not found",
            "job_id": job_id
        })));
    }
    
    let (res, mut session, mut stream) = actix_ws::handle(&req, stream)?;
    
    info!("🔌 WebSocket connection for job: {}", job_id);
    
    // Canal para updates específicos de este job
    let (tx, mut rx) = mpsc::unbounded_channel::<JobProgressUpdate>();
    
    // Registrar en job manager
    job_manager.register_websocket(&job_id, tx).await;
    
    // Crear tarea para manejar la conexión
    let job_manager_clone = job_manager.get_ref().clone();
    let job_id_clone = job_id.clone();
    
    actix_web::rt::spawn(async move {
        // Enviar estado actual del job
        if let Some(job) = job_manager_clone.get_job(&job_id_clone).await {
            let update = JobProgressUpdate {
                job_id: job.job_id,
                status: job.status,
                progress_percent: job.progress_percent,
                current_step: job.current_step,
                poses_evaluated: job.computation_stats.as_ref()
                    .map(|s| s.total_poses_evaluated)
                    .unwrap_or(0),
                poses_remaining: job.config.max_poses.saturating_sub(
                    job.computation_stats.as_ref()
                        .map(|s| s.total_poses_evaluated)
                        .unwrap_or(0)
                ),
                estimated_completion: job.estimated_completion,
                computation_stats: job.computation_stats,
                timestamp: chrono::Utc::now(),
            };
            
            let _ = send_websocket_message(
                &mut session,
                &WebSocketMessage::JobProgress(update)
            ).await;
        }
        
        // Loop principal
        loop {
            tokio::select! {
                // Mensaje del cliente
                msg = stream.next() => {
                    match msg {
                        Some(Ok(AggregatedMessage::Text(text))) => {
                            debug!("Received message: {}", text);
                            // En este caso simple, solo manejamos ping/pong
                            if text.trim() == "ping" {
                                let _ = session.text("pong").await;
                            }
                        },
                        Some(Ok(AggregatedMessage::Ping(payload))) => {
                            let _ = session.pong(&payload).await;
                        },
                        Some(Ok(AggregatedMessage::Close(_))) => {
                            info!("Client closed WebSocket for job: {}", job_id_clone);
                            break;
                        },
                        Some(Err(e)) => {
                            error!("WebSocket error for job {}: {}", job_id_clone, e);
                            break;
                        },
                        None => break,
                        _ => {}
                    }
                },
                
                // Update de progreso
                update = rx.recv() => {
                    if let Some(update) = update {
                        if let Err(e) = send_websocket_message(
                            &mut session,
                            &WebSocketMessage::JobProgress(update)
                        ).await {
                            error!("Failed to send update for job {}: {}", job_id_clone, e);
                            break;
                        }
                    } else {
                        debug!("Update channel closed for job: {}", job_id_clone);
                        break;
                    }
                }
            }
        }
        
        // Cleanup
        job_manager_clone.unregister_websocket(&job_id_clone).await;
        info!("🔌 WebSocket disconnected for job: {}", job_id_clone);
    });
    
    Ok(res)
}

/// Handler para obtener estado de conexiones WebSocket
pub async fn websocket_status(
    job_manager: web::Data<Arc<JobManager>>,
) -> Result<HttpResponse> {
    let connections_count = job_manager.websocket_connections.read().await.len();
    let active_jobs = job_manager.active_jobs.read().await.len();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "websocket_connections": connections_count,
        "active_jobs": active_jobs,
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    })))
}

// Tests para WebSocket (opcional)
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_websocket_message_serialization() {
        let message = WebSocketMessage::Subscribe {
            job_id: "test-job-123".to_string()
        };
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            WebSocketMessage::Subscribe { job_id } => {
                assert_eq!(job_id, "test-job-123");
            },
            _ => panic!("Wrong message type")
        }
    }
}