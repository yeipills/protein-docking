//! HTTP API.

use std::convert::Infallible;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use cs_core::Mode;
use cs_core::corrected::Params;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::db::Status;
use super::jobs::{INPUT_DIR, INPUT_FACE, INPUT_STL, INPUT_VERT, Jobs, OUTPUT_DIR};
use crate::pipeline::{Spec, sanitize_name};

pub struct AppState {
    pub jobs: Arc<Jobs>,
    pub token: Option<String>,
    pub samples_dir: PathBuf,
}

type St = State<Arc<AppState>>;

pub struct ApiError(StatusCode, String);

impl ApiError {
    fn bad(msg: impl Into<String>) -> Self {
        ApiError(StatusCode::BAD_REQUEST, msg.into())
    }
    fn not_found() -> Self {
        ApiError(StatusCode::NOT_FOUND, "not found".into())
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        let e = e.into();
        tracing::error!("internal error: {e:#}");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

pub fn router(state: Arc<AppState>, max_upload_bytes: usize) -> Router {
    let api = Router::new()
        .route("/config", get(config))
        .route("/jobs", get(list_jobs).post(create_job))
        .route("/jobs/{id}", get(get_job).delete(delete_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
        .route("/jobs/{id}/events", get(job_events))
        .route("/jobs/{id}/files/{file}", get(job_file))
        .route("/jobs/{id}/archive", get(job_archive))
        .route("/samples", get(list_samples))
        .route("/samples/{name}/jobs", post(create_sample_job))
        .layer(DefaultBodyLimit::max(max_upload_bytes))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth));
    Router::new()
        .route("/api/health", get(health))
        .nest("/api", api)
        .with_state(state)
}

/// Requires `Authorization: Bearer <token>` (or `?token=` for EventSource and downloads)
/// when the server was started with a token.
async fn auth(State(st): St, req: Request, next: Next) -> Response {
    let Some(expected) = &st.token else {
        return next.run(req).await;
    };
    let from_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string);
    let from_query = req.uri().query().and_then(|q| {
        q.split('&')
            .find_map(|kv| kv.strip_prefix("token="))
            .map(percent_decode)
    });
    let ok = from_header
        .or(from_query)
        .is_some_and(|t| constant_time_eq(t.as_bytes(), expected.as_bytes()));
    if ok {
        next.run(req).await
    } else {
        ApiError(
            StatusCode::UNAUTHORIZED,
            "missing or invalid access token".into(),
        )
        .into_response()
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
                continue;
            }
            (b'+', _) => out.push(b' '),
            (b, _) => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

#[derive(Serialize)]
struct Config {
    version: &'static str,
    defaults: Params,
}

async fn config() -> Json<Config> {
    Json(Config {
        version: env!("CARGO_PKG_VERSION"),
        defaults: Params::default(),
    })
}

async fn list_jobs(State(st): St) -> ApiResult<Json<Vec<super::db::Job>>> {
    Ok(Json(st.jobs.db.list()?))
}

async fn get_job(State(st): St, Path(id): Path<String>) -> ApiResult<Json<super::db::Job>> {
    let mut job = st.jobs.db.get(&id)?.ok_or_else(ApiError::not_found)?;
    // Live progress is fresher than the throttled database copy.
    if let Some((ev, _)) = st.jobs.subscribe(&id) {
        job.status = ev.status;
        job.progress = ev.progress;
        job.stage = ev.stage;
    }
    Ok(Json(job))
}

fn valid_id(id: &str) -> ApiResult<()> {
    if id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}

async fn cancel_job(State(st): St, Path(id): Path<String>) -> ApiResult<StatusCode> {
    valid_id(&id)?;
    if st.jobs.cancel(&id) {
        Ok(StatusCode::ACCEPTED)
    } else if st.jobs.db.get(&id)?.is_some() {
        Err(ApiError(
            StatusCode::CONFLICT,
            "the job has already finished".into(),
        ))
    } else {
        Err(ApiError::not_found())
    }
}

async fn delete_job(State(st): St, Path(id): Path<String>) -> ApiResult<StatusCode> {
    valid_id(&id)?;
    if st.jobs.cancel(&id) {
        // Give a running job a moment to stop before removing its files.
        for _ in 0..50 {
            if st.jobs.subscribe(&id).is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    let jobs = st.jobs.clone();
    let existed = tokio::task::spawn_blocking(move || jobs.delete(&id)).await??;
    if existed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found())
    }
}

async fn job_events(
    State(st): St,
    Path(id): Path<String>,
) -> ApiResult<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>> {
    valid_id(&id)?;
    let to_event = |e: &super::jobs::JobEvent| Event::default().json_data(e).unwrap();
    let (first, rx) = match st.jobs.subscribe(&id) {
        Some((ev, rx)) => (ev, Some(rx)),
        None => {
            let job = st.jobs.db.get(&id)?.ok_or_else(ApiError::not_found)?;
            let ev = super::jobs::JobEvent {
                status: job.status,
                progress: job.progress,
                stage: job.stage,
                error: job.error,
            };
            (ev, None)
        }
    };
    // Forward events until the job reaches a terminal state, then end the stream.
    let (tx, out) = tokio::sync::mpsc::channel::<super::jobs::JobEvent>(16);
    let done = first.status.is_terminal();
    let _ = tx.try_send(first);
    if let (Some(rx), false) = (rx, done) {
        tokio::spawn(async move {
            let mut events = BroadcastStream::new(rx);
            while let Some(item) = events.next().await {
                let Ok(ev) = item else { continue };
                let terminal = ev.status.is_terminal();
                if tx.send(ev).await.is_err() || terminal {
                    break;
                }
            }
        });
    }
    let stream = tokio_stream::wrappers::ReceiverStream::new(out).map(move |e| Ok(to_event(&e)));
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Serves one output file. `?download` adds a Content-Disposition header.
async fn job_file(
    State(st): St,
    Path((id, file)): Path<(String, String)>,
    req: Request,
) -> ApiResult<Response> {
    valid_id(&id)?;
    if file.contains('/') || file.contains('\\') || file.starts_with('.') {
        return Err(ApiError::not_found());
    }
    let path = st.jobs.job_dir(&id).join(OUTPUT_DIR).join(&file);
    if !path.is_file() {
        return Err(ApiError::not_found());
    }
    let download = req
        .uri()
        .query()
        .is_some_and(|q| q.split('&').any(|kv| kv == "download"));
    let mime = match FsPath::new(&file).extension().and_then(|e| e.to_str()) {
        Some("txt") => "text/plain; charset=utf-8",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    };
    serve_file(&path, req, mime, download.then_some(file.as_str())).await
}

async fn serve_file(
    path: &FsPath,
    req: Request,
    mime: &str,
    attachment: Option<&str>,
) -> ApiResult<Response> {
    let mut res = ServeFile::new_with_mime(path, &mime.parse().unwrap())
        .oneshot(req)
        .await?
        .map(Body::new);
    if let Some(name) = attachment {
        res.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("attachment; filename=\"{name}\""))?,
        );
    }
    Ok(res)
}

/// Zip of all output files, built on first request and cached next to the outputs.
async fn job_archive(State(st): St, Path(id): Path<String>, req: Request) -> ApiResult<Response> {
    valid_id(&id)?;
    let job = st.jobs.db.get(&id)?.ok_or_else(ApiError::not_found)?;
    if job.status != Status::Done {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "the job has not finished".into(),
        ));
    }
    let dir = st.jobs.job_dir(&id);
    let zip_path = dir.join("results.zip");
    if !zip_path.exists() {
        let (out, tmp, target) = (
            dir.join(OUTPUT_DIR),
            dir.join("results.zip.tmp"),
            zip_path.clone(),
        );
        tokio::task::spawn_blocking(move || build_zip(&out, &tmp, &target)).await??;
    }
    let name = format!("{}_results.zip", sanitize_name(&job.name));
    serve_file(&zip_path, req, "application/zip", Some(&name)).await
}

fn build_zip(src: &FsPath, tmp: &FsPath, target: &FsPath) -> anyhow::Result<()> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(std::fs::File::create(tmp)?);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);
    let mut entries: Vec<_> = std::fs::read_dir(src)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if e.file_type()?.is_file() {
            zip.start_file(e.file_name().to_string_lossy(), opts)?;
            zip.write_all(&std::fs::read(e.path())?)?;
        }
    }
    zip.finish()?;
    std::fs::rename(tmp, target)?;
    Ok(())
}

// ------------------------------------------------------------------------------------------
// Job creation

#[derive(Debug, Default, Deserialize)]
struct Options {
    name: Option<String>,
    mode: Option<Mode>,
    params: Option<Params>,
    #[serde(default)]
    legacy_intermediate: bool,
}

fn build_spec(opts: Options, fallback_name: &str) -> ApiResult<Spec> {
    let params = opts.params.unwrap_or_default();
    params
        .validate()
        .map_err(|e| ApiError::bad(e.to_string()))?;
    let name = opts
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| fallback_name.into());
    Ok(Spec {
        name: sanitize_name(&name),
        mode: opts.mode.unwrap_or(Mode::Corrected),
        params,
        legacy_intermediate: opts.legacy_intermediate,
    })
}

/// Parses the uploaded MSMS files before queueing so mistakes are reported immediately.
async fn check_inputs(input: PathBuf, spec: &Spec) -> ApiResult<()> {
    let needs_stl = spec.mode == Mode::Legacy;
    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let s =
            cs_core::io::msms::MsmsSurface::read(&input.join(INPUT_VERT), &input.join(INPUT_FACE))
                .map_err(|e| e.to_string())?;
        s.faces().map_err(|e| e.to_string())?;
        let stl = input.join(INPUT_STL);
        if needs_stl {
            if !stl.exists() {
                return Err("legacy mode needs the surface STL".into());
            }
            cs_core::io::stl::Stl::read(&stl).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await?;
    res.map_err(ApiError::bad)
}

/// `multipart/form-data` with files `vert`, `face`, optional `stl`, and optional text fields
/// `name`, `mode` (`corrected` | `legacy`), `params` (JSON) and `legacy_intermediate`.
async fn create_job(
    State(st): St,
    mut form: Multipart,
) -> ApiResult<(StatusCode, Json<super::db::Job>)> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = st.jobs.job_dir(&id);
    let input = dir.join(INPUT_DIR);
    tokio::fs::create_dir_all(&input).await?;
    let result = async {
        let mut opts = Options::default();
        let mut vert_name = None;
        let (mut has_vert, mut has_face) = (false, false);
        while let Some(mut field) = form
            .next_field()
            .await
            .map_err(|e| ApiError::bad(e.to_string()))?
        {
            let field_name = field.name().unwrap_or_default().to_string();
            let target = match field_name.as_str() {
                "vert" => Some(INPUT_VERT),
                "face" => Some(INPUT_FACE),
                "stl" => Some(INPUT_STL),
                _ => None,
            };
            if let Some(target) = target {
                if field_name == "vert" {
                    vert_name = field
                        .file_name()
                        .map(|f| f.trim_end_matches(".vert").to_string());
                }
                let mut file = tokio::fs::File::create(input.join(target)).await?;
                let mut size = 0usize;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| ApiError::bad(e.to_string()))?
                {
                    size += chunk.len();
                    file.write_all(&chunk).await?;
                }
                file.flush().await?;
                if size == 0 {
                    // An empty optional file input is the same as no file.
                    tokio::fs::remove_file(input.join(target)).await?;
                    continue;
                }
                has_vert |= field_name == "vert";
                has_face |= field_name == "face";
                continue;
            }
            let text = field
                .text()
                .await
                .map_err(|e| ApiError::bad(e.to_string()))?;
            match field_name.as_str() {
                "name" => opts.name = Some(text),
                "mode" => {
                    opts.mode = Some(
                        serde_json::from_value(serde_json::Value::String(text.clone()))
                            .map_err(|_| ApiError::bad(format!("unknown mode {text:?}")))?,
                    )
                }
                "params" if !text.trim().is_empty() => {
                    opts.params = Some(
                        serde_json::from_str(&text)
                            .map_err(|e| ApiError::bad(format!("params: {e}")))?,
                    )
                }
                "legacy_intermediate" => {
                    opts.legacy_intermediate = matches!(text.as_str(), "true" | "1" | "on")
                }
                _ => {}
            }
        }
        if !has_vert || !has_face {
            return Err(ApiError::bad("both the .vert and .face files are required"));
        }
        let spec = build_spec(opts, vert_name.as_deref().unwrap_or("protein"))?;
        check_inputs(input.clone(), &spec).await?;
        st.jobs.submit(&id, &spec)?;
        Ok(())
    }
    .await;
    if let Err(e) = result {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Err(e);
    }
    let job = st.jobs.db.get(&id)?.ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(job)))
}

#[derive(Serialize)]
struct Sample {
    name: String,
    has_stl: bool,
}

/// A sample is a directory `<samples>/<name>` with `<name>.vert`, `<name>.face` and
/// optionally `<name>.stl`.
fn sample_files(dir: &FsPath, name: &str) -> Option<(PathBuf, PathBuf, Option<PathBuf>)> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return None;
    }
    let d = dir.join(name);
    let (v, f, s) = (
        d.join(format!("{name}.vert")),
        d.join(format!("{name}.face")),
        d.join(format!("{name}.stl")),
    );
    (v.is_file() && f.is_file()).then(|| (v, f, s.is_file().then_some(s)))
}

async fn list_samples(State(st): St) -> ApiResult<Json<Vec<Sample>>> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&st.samples_dir) {
        for e in rd.filter_map(|e| e.ok()) {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some((_, _, stl)) = sample_files(&st.samples_dir, &name) {
                out.push(Sample {
                    name,
                    has_stl: stl.is_some(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(out))
}

async fn create_sample_job(
    State(st): St,
    Path(name): Path<String>,
    body: Option<Json<serde_json::Value>>,
) -> ApiResult<(StatusCode, Json<super::db::Job>)> {
    let (vert, face, stl) = sample_files(&st.samples_dir, &name).ok_or_else(ApiError::not_found)?;
    let opts: Options = match body {
        Some(Json(v)) => serde_json::from_value(v).map_err(|e| ApiError::bad(e.to_string()))?,
        None => Options::default(),
    };
    let spec = build_spec(
        Options {
            name: opts.name.clone().or_else(|| Some(name.clone())),
            ..opts
        },
        &name,
    )?;
    if spec.mode == Mode::Legacy && stl.is_none() {
        return Err(ApiError::bad(
            "this sample has no STL, which legacy mode needs",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let input = st.jobs.job_dir(&id).join(INPUT_DIR);
    tokio::fs::create_dir_all(&input).await?;
    tokio::fs::copy(&vert, input.join(INPUT_VERT)).await?;
    tokio::fs::copy(&face, input.join(INPUT_FACE)).await?;
    if let (Some(stl), Mode::Legacy) = (&stl, spec.mode) {
        tokio::fs::copy(stl, input.join(INPUT_STL)).await?;
    }
    st.jobs.submit(&id, &spec)?;
    let job = st.jobs.db.get(&id)?.ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(job)))
}
