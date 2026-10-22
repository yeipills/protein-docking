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
use super::jobs::{
    INPUT_DIR, INPUT_FACE, INPUT_LIGAND, INPUT_NATIVE_LIGAND, INPUT_NATIVE_RECEPTOR, INPUT_PDB,
    INPUT_RECEPTOR, INPUT_STL, INPUT_VERT, Jobs, OUTPUT_DIR,
};
use crate::docking::{DockSpec, DockSurface};
use crate::pipeline::{Spec, sanitize_name};
use crate::task::Task;

pub struct AppState {
    pub jobs: Arc<Jobs>,
    pub token: Option<String>,
    pub samples_dir: PathBuf,
    pub complexes_dir: PathBuf,
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
        .route("/dock", post(create_dock_job))
        .route("/complexes", get(list_complexes))
        .route("/complexes/{name}/dock", post(create_complex_dock_job))
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
    surface: Option<cs_core::surface::SesParams>,
}

fn build_spec(opts: Options, fallback_name: &str) -> ApiResult<Spec> {
    let params = opts.params.unwrap_or_default();
    params
        .validate()
        .map_err(|e| ApiError::bad(e.to_string()))?;
    let surface = opts.surface.unwrap_or_default();
    surface
        .validate()
        .map_err(|e| ApiError::bad(format!("surface: {e}")))?;
    let name = opts
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| fallback_name.into());
    Ok(Spec {
        name: sanitize_name(&name),
        mode: opts.mode.unwrap_or(Mode::Corrected),
        params,
        legacy_intermediate: opts.legacy_intermediate,
        surface,
    })
}

/// Parses the uploaded files before queueing so mistakes are reported immediately.
async fn check_inputs(input: PathBuf, spec: &Spec) -> ApiResult<()> {
    let needs_stl = spec.mode == Mode::Legacy;
    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let pdb = input.join(INPUT_PDB);
        if pdb.exists() {
            if needs_stl {
                return Err(
                    "legacy mode needs MSMS files and an STL; use the corrected mode for PDB input"
                        .into(),
                );
            }
            cs_core::io::pdb::Structure::read(&pdb).map_err(|e| e.to_string())?;
            return Ok(());
        }
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

/// `multipart/form-data` with either a `pdb` file or files `vert` + `face` (and `stl` for
/// legacy mode), plus optional text fields `name`, `mode` (`corrected` | `legacy`),
/// `params` (JSON), `surface` (JSON, for PDB input) and `legacy_intermediate`.
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
        let mut file_name = None;
        let (mut has_vert, mut has_face, mut has_pdb) = (false, false, false);
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
                "pdb" => Some(INPUT_PDB),
                _ => None,
            };
            if let Some(target) = target {
                if field_name == "vert" || field_name == "pdb" {
                    file_name = field.file_name().map(|f| {
                        let f = f.rsplit(['/', '\\']).next().unwrap_or(f);
                        f.rsplit_once('.').map_or(f, |(stem, _)| stem).to_string()
                    });
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
                has_pdb |= field_name == "pdb";
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
                "surface" if !text.trim().is_empty() => {
                    opts.surface = Some(
                        serde_json::from_str(&text)
                            .map_err(|e| ApiError::bad(format!("surface: {e}")))?,
                    )
                }
                "legacy_intermediate" => {
                    opts.legacy_intermediate = matches!(text.as_str(), "true" | "1" | "on")
                }
                _ => {}
            }
        }
        if has_pdb && (has_vert || has_face) {
            return Err(ApiError::bad(
                "send either a PDB file or the MSMS .vert/.face files, not both",
            ));
        }
        if !has_pdb && (!has_vert || !has_face) {
            return Err(ApiError::bad(
                "a PDB file, or both the .vert and .face files, are required",
            ));
        }
        let spec = build_spec(opts, file_name.as_deref().unwrap_or("protein"))?;
        check_inputs(input.clone(), &spec).await?;
        st.jobs.submit(&id, &Task::Shapes(spec))?;
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
    st.jobs.submit(&id, &Task::Shapes(spec))?;
    let job = st.jobs.db.get(&id)?.ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(job)))
}

// ------------------------------------------------------------------------------------------
// Docking jobs

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DockOptions {
    name: Option<String>,
    surface: Option<DockSurface>,
    params: Option<cs_core::dock::DockParams>,
    randomize_seed: Option<u64>,
    complexes: Option<usize>,
}

fn build_dock_spec(opts: DockOptions, fallback_name: &str) -> ApiResult<DockSpec> {
    let params = opts.params.unwrap_or_default();
    params
        .validate()
        .map_err(|e| ApiError::bad(format!("params: {e}")))?;
    let name = opts
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| fallback_name.into());
    Ok(DockSpec {
        name: sanitize_name(&name),
        surface: opts.surface.unwrap_or_default(),
        params,
        randomize_seed: opts.randomize_seed,
        complexes: opts.complexes.unwrap_or(10).min(100),
    })
}

/// Parses the uploaded structures (and the native pair, which must come together).
async fn check_dock_inputs(input: PathBuf) -> ApiResult<()> {
    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        for (file, what) in [(INPUT_RECEPTOR, "receptor"), (INPUT_LIGAND, "ligand")] {
            cs_core::io::pdb::Structure::read(&input.join(file))
                .map_err(|e| format!("{what}: {e}"))?;
        }
        let (nr, nl) = (
            input.join(INPUT_NATIVE_RECEPTOR),
            input.join(INPUT_NATIVE_LIGAND),
        );
        match (nr.exists(), nl.exists()) {
            (false, false) => Ok(()),
            (true, true) => {
                let read =
                    |p: &FsPath| cs_core::io::pdb::Structure::read(p).map_err(|e| e.to_string());
                let (r, l) = (
                    read(&input.join(INPUT_RECEPTOR))?,
                    read(&input.join(INPUT_LIGAND))?,
                );
                cs_core::eval::Native::new(&read(&nr)?, &read(&nl)?, &r, &l)
                    .map(|_| ())
                    .map_err(|e| format!("native complex: {e}"))
            }
            _ => Err("send both native_receptor and native_ligand, or neither".into()),
        }
    })
    .await?;
    res.map_err(ApiError::bad)
}

/// `multipart/form-data` with files `receptor` and `ligand` (PDB), optionally
/// `native_receptor` + `native_ligand` (the bound complex, to score poses with CAPRI/DockQ),
/// and text fields `name`, `surface` (`atoms` | `ses`), `params` (JSON), `randomize_seed`
/// and `complexes`.
async fn create_dock_job(
    State(st): St,
    mut form: Multipart,
) -> ApiResult<(StatusCode, Json<super::db::Job>)> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = st.jobs.job_dir(&id);
    let input = dir.join(INPUT_DIR);
    tokio::fs::create_dir_all(&input).await?;
    let result = async {
        let mut opts = DockOptions::default();
        let mut names: (Option<String>, Option<String>) = (None, None);
        while let Some(mut field) = form
            .next_field()
            .await
            .map_err(|e| ApiError::bad(e.to_string()))?
        {
            let field_name = field.name().unwrap_or_default().to_string();
            let target = match field_name.as_str() {
                "receptor" => Some(INPUT_RECEPTOR),
                "ligand" => Some(INPUT_LIGAND),
                "native_receptor" => Some(INPUT_NATIVE_RECEPTOR),
                "native_ligand" => Some(INPUT_NATIVE_LIGAND),
                _ => None,
            };
            if let Some(target) = target {
                let stem = field.file_name().map(|f| {
                    let f = f.rsplit(['/', '\\']).next().unwrap_or(f);
                    f.rsplit_once('.').map_or(f, |(stem, _)| stem).to_string()
                });
                match field_name.as_str() {
                    "receptor" => names.0 = stem,
                    "ligand" => names.1 = stem,
                    _ => {}
                }
                let path = input.join(target);
                let mut file = tokio::fs::File::create(&path).await?;
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
                    tokio::fs::remove_file(&path).await?;
                }
                continue;
            }
            let text = field
                .text()
                .await
                .map_err(|e| ApiError::bad(e.to_string()))?;
            let text = text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            match field_name.as_str() {
                "name" => opts.name = Some(text),
                "surface" => {
                    opts.surface = Some(
                        serde_json::from_value(serde_json::Value::String(text.clone()))
                            .map_err(|_| ApiError::bad(format!("unknown surface {text:?}")))?,
                    )
                }
                "params" => {
                    opts.params = Some(
                        serde_json::from_str(&text)
                            .map_err(|e| ApiError::bad(format!("params: {e}")))?,
                    )
                }
                "randomize_seed" => {
                    opts.randomize_seed = Some(
                        text.parse()
                            .map_err(|_| ApiError::bad("randomize_seed must be an integer"))?,
                    )
                }
                "complexes" => {
                    opts.complexes = Some(
                        text.parse()
                            .map_err(|_| ApiError::bad("complexes must be an integer"))?,
                    )
                }
                _ => {}
            }
        }
        if !input.join(INPUT_RECEPTOR).exists() || !input.join(INPUT_LIGAND).exists() {
            return Err(ApiError::bad(
                "both the receptor and the ligand PDB files are required",
            ));
        }
        let fallback = match names {
            (Some(r), Some(l)) => format!("{r}+{l}"),
            _ => "docking".into(),
        };
        let spec = build_dock_spec(opts, &fallback)?;
        check_dock_inputs(input.clone()).await?;
        st.jobs.submit(&id, &Task::Dock(spec))?;
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
struct Complex {
    name: String,
    has_native: bool,
}

/// A complex is a directory `<complexes>/<name>` with `<name>_r_u.pdb` and `<name>_l_u.pdb`,
/// and optionally the bound pair `<name>_r_b.pdb` / `<name>_l_b.pdb` (Docking Benchmark naming).
fn complex_files(
    dir: &FsPath,
    name: &str,
) -> Option<(PathBuf, PathBuf, Option<(PathBuf, PathBuf)>)> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return None;
    }
    let d = dir.join(name);
    let f = |s: &str| d.join(format!("{name}_{s}.pdb"));
    let (ru, lu, rb, lb) = (f("r_u"), f("l_u"), f("r_b"), f("l_b"));
    (ru.is_file() && lu.is_file())
        .then(|| (ru, lu, (rb.is_file() && lb.is_file()).then_some((rb, lb))))
}

async fn list_complexes(State(st): St) -> ApiResult<Json<Vec<Complex>>> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&st.complexes_dir) {
        for e in rd.filter_map(|e| e.ok()) {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some((_, _, native)) = complex_files(&st.complexes_dir, &name) {
                out.push(Complex {
                    name,
                    has_native: native.is_some(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(out))
}

/// Docks a bundled complex. The unbound files of the Docking Benchmark are superposed on
/// the complex, so the ligand starts from a random placement unless `randomize_seed` says
/// otherwise; the bound pair, when present, scores the poses.
async fn create_complex_dock_job(
    State(st): St,
    Path(name): Path<String>,
    body: Option<Json<serde_json::Value>>,
) -> ApiResult<(StatusCode, Json<super::db::Job>)> {
    let (ru, lu, native) =
        complex_files(&st.complexes_dir, &name).ok_or_else(ApiError::not_found)?;
    let mut opts: DockOptions = match body {
        Some(Json(v)) => serde_json::from_value(v).map_err(|e| ApiError::bad(e.to_string()))?,
        None => DockOptions::default(),
    };
    // Same start as the benchmark harness (seed 1), so results match docs/DOCKING.md.
    opts.randomize_seed = opts
        .randomize_seed
        .or(Some(cs_core::eval::bench::case_seed(1, &name)));
    let spec = build_dock_spec(opts, &name)?;
    let id = uuid::Uuid::new_v4().to_string();
    let input = st.jobs.job_dir(&id).join(INPUT_DIR);
    tokio::fs::create_dir_all(&input).await?;
    tokio::fs::copy(&ru, input.join(INPUT_RECEPTOR)).await?;
    tokio::fs::copy(&lu, input.join(INPUT_LIGAND)).await?;
    if let Some((rb, lb)) = &native {
        tokio::fs::copy(rb, input.join(INPUT_NATIVE_RECEPTOR)).await?;
        tokio::fs::copy(lb, input.join(INPUT_NATIVE_LIGAND)).await?;
    }
    st.jobs.submit(&id, &Task::Dock(spec))?;
    let job = st.jobs.db.get(&id)?.ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(job)))
}
