//! HTTP server: REST API, job queue and the static web client.

pub mod api;
pub mod db;
pub mod jobs;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

pub struct ServeOptions {
    pub addr: SocketAddr,
    pub data_dir: PathBuf,
    pub web_dir: Option<PathBuf>,
    pub samples_dir: PathBuf,
    pub complexes_dir: PathBuf,
    pub token: Option<String>,
    pub workers: usize,
    pub max_upload_mb: usize,
}

pub async fn build(opts: &ServeOptions) -> anyhow::Result<Router> {
    std::fs::create_dir_all(&opts.data_dir)
        .with_context(|| format!("creating {}", opts.data_dir.display()))?;
    let db = Arc::new(db::Db::open(&opts.data_dir.join("cshape.db"))?);
    let jobs = jobs::Jobs::start(db, opts.data_dir.clone(), opts.workers)?;
    let state = Arc::new(api::AppState {
        jobs,
        token: opts.token.clone().filter(|t| !t.is_empty()),
        samples_dir: opts.samples_dir.clone(),
        complexes_dir: opts.complexes_dir.clone(),
    });
    let mut app = api::router(state, opts.max_upload_mb * 1024 * 1024);
    if let Some(web) = &opts.web_dir {
        let index = web.join("index.html");
        if index.is_file() {
            app = app.fallback_service(ServeDir::new(web).fallback(ServeFile::new(index)));
        } else {
            tracing::warn!("{} has no index.html; serving the API only", web.display());
        }
    }
    Ok(app
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http()))
}

pub async fn serve(opts: ServeOptions) -> anyhow::Result<()> {
    let app = build(&opts).await?;
    let listener = tokio::net::TcpListener::bind(opts.addr)
        .await
        .with_context(|| format!("binding {}", opts.addr))?;
    tracing::info!(
        "listening on http://{} (data: {}, auth: {})",
        opts.addr,
        opts.data_dir.display(),
        if opts.token.as_deref().is_some_and(|t| !t.is_empty()) {
            "token"
        } else {
            "none"
        }
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Resolves on Ctrl-C or SIGTERM (what `docker stop` sends).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = term => {}
    }
    tracing::info!("shutting down");
}
