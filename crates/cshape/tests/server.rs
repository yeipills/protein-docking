//! API tests against the in-process router.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use cshape::server::{ServeOptions, build};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::Digest;
use tower::ServiceExt;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

async fn app(data: &tempfile::TempDir, token: Option<&str>) -> Router {
    build(&ServeOptions {
        addr: "127.0.0.1:0".parse().unwrap(),
        data_dir: data.path().to_path_buf(),
        web_dir: None,
        samples_dir: repo().join("data/samples"),
        complexes_dir: repo().join("data/complexes"),
        token: token.map(str::to_string),
        workers: 1,
        max_upload_mb: 64,
    })
    .await
    .unwrap()
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Vec<u8>, header::HeaderMap) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, body, headers)
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let (s, b, _) = send(app, Request::get(uri).body(Body::empty()).unwrap()).await;
    (s, b)
}

async fn get_json(app: &Router, uri: &str) -> Value {
    let (s, b) = get(app, uri).await;
    assert_eq!(s, StatusCode::OK, "{uri}: {}", String::from_utf8_lossy(&b));
    serde_json::from_slice(&b).unwrap()
}

async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let (s, b, _) = send(app, req).await;
    (s, serde_json::from_slice(&b).unwrap_or(Value::Null))
}

async fn wait_done(app: &Router, id: &str) -> Value {
    let start = Instant::now();
    loop {
        let job = get_json(app, &format!("/api/jobs/{id}")).await;
        match job["status"].as_str().unwrap() {
            "done" => return job,
            "failed" | "cancelled" => panic!("job ended as {job}"),
            _ => {}
        }
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "job did not finish"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn multipart(parts: &[(&str, Option<&str>, Vec<u8>)]) -> (String, Vec<u8>) {
    let boundary = "----cshape-test-boundary";
    let mut body = Vec::new();
    for (name, filename, data) in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match filename {
            Some(f) => body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{name}\"; filename=\"{f}\"\r\n\r\n"
                )
                .as_bytes(),
            ),
            None => body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
            ),
        }
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

fn oracle_hash(file: &str) -> String {
    let list =
        std::fs::read_to_string(repo().join("crates/cs-core/tests/legacy_oracle.sha256")).unwrap();
    list.lines()
        .find_map(|l| {
            l.split_once("  ")
                .filter(|(_, n)| *n == file)
                .map(|(h, _)| h.to_string())
        })
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn sample_jobs_end_to_end() {
    let data = tempfile::tempdir().unwrap();
    let app = app(&data, None).await;

    assert_eq!(get(&app, "/api/health").await.0, StatusCode::OK);
    let samples = get_json(&app, "/api/samples").await;
    assert_eq!(samples[0]["name"], "1AHW_l_u");

    // Corrected mode.
    let (s, job) = post_json(
        &app,
        "/api/samples/1AHW_l_u/jobs",
        json!({ "mode": "corrected" }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{job}");
    let id = job["id"].as_str().unwrap().to_string();
    let job = wait_done(&app, &id).await;
    assert_eq!(job["report"]["summary"]["mode"], "corrected");
    assert!(job["report"]["summary"]["context_shapes"].as_u64().unwrap() > 0);

    let (s, blob) = get(&app, &format!("/api/jobs/{id}/files/viewer.bin")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(&blob[..4], b"CSV1");

    // Events of a finished job: a single terminal event.
    let (s, body) = get(&app, &format!("/api/jobs/{id}/events")).await;
    assert_eq!(s, StatusCode::OK);
    let text = String::from_utf8(body).unwrap();
    assert!(
        text.contains("data:") && text.contains("\"status\":\"done\""),
        "{text}"
    );

    // Archive.
    let (s, zip, headers) = send(
        &app,
        Request::get(format!("/api/jobs/{id}/archive"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(&zip[..2], b"PK");
    assert!(
        headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("1AHW_l_u_results.zip")
    );

    // Legacy mode reproduces the original scripts through the API too.
    let (s, job) = post_json(
        &app,
        "/api/samples/1AHW_l_u/jobs",
        json!({ "mode": "legacy" }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{job}");
    let legacy_id = job["id"].as_str().unwrap().to_string();
    wait_done(&app, &legacy_id).await;
    let (s, ses) = get(
        &app,
        &format!("/api/jobs/{legacy_id}/files/1AHW_l_u_cs_ses_unity.txt"),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let hash: String = sha2::Sha256::digest(&ses)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hash, oracle_hash("1AHW_l_u_cs_ses_unity.txt"));

    // Listing, path traversal, deletion.
    assert_eq!(
        get_json(&app, "/api/jobs").await.as_array().unwrap().len(),
        2
    );
    assert_eq!(
        get(&app, &format!("/api/jobs/{id}/files/..%2Fcshape.db"))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (s, _, _) = send(
        &app,
        Request::delete(format!("/api/jobs/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(
        get(&app, &format!("/api/jobs/{id}")).await.0,
        StatusCode::NOT_FOUND
    );
    assert!(!data.path().join("jobs").join(&id).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn upload_validation() {
    let data = tempfile::tempdir().unwrap();
    let app = app(&data, None).await;
    let dir = repo().join("data/samples/1AHW_l_u");
    let vert = std::fs::read(dir.join("1AHW_l_u.vert")).unwrap();
    let face = std::fs::read(dir.join("1AHW_l_u.face")).unwrap();

    let upload = |parts: Vec<(&'static str, Option<&'static str>, Vec<u8>)>| {
        let (ct, body) = multipart(&parts);
        Request::post("/api/jobs")
            .header(header::CONTENT_TYPE, ct)
            .body(Body::from(body))
            .unwrap()
    };

    // Missing face file.
    let (s, b, _) = send(&app, upload(vec![("vert", Some("x.vert"), vert.clone())])).await;
    assert_eq!(
        s,
        StatusCode::BAD_REQUEST,
        "{}",
        String::from_utf8_lossy(&b)
    );

    // Legacy without STL.
    let (s, _, _) = send(
        &app,
        upload(vec![
            ("vert", Some("x.vert"), vert.clone()),
            ("face", Some("x.face"), face.clone()),
            ("mode", None, b"legacy".to_vec()),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Garbage file.
    let (s, b, _) = send(
        &app,
        upload(vec![
            ("vert", Some("x.vert"), b"a\nb\nc\nnot numbers\n".to_vec()),
            ("face", Some("x.face"), face.clone()),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&b).contains("vert"));

    // Bad parameters.
    let (s, _, _) = send(
        &app,
        upload(vec![
            ("vert", Some("x.vert"), vert.clone()),
            ("face", Some("x.face"), face.clone()),
            ("params", None, br#"{"rays": 0}"#.to_vec()),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Valid upload with custom parameters; the name comes from the file name.
    let (s, b, _) = send(
        &app,
        upload(vec![
            ("vert", Some("myprot.vert"), vert),
            ("face", Some("myprot.face"), face),
            (
                "params",
                None,
                br#"{"rays": 50, "samples": 8, "min_spacing": 12}"#.to_vec(),
            ),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let job: Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(job["name"], "myprot");
    let job = wait_done(&app, job["id"].as_str().unwrap()).await;
    assert_eq!(job["report"]["summary"]["rays_per_shape"], 50);
    assert_eq!(job["report"]["summary"]["samples_per_ray"], 8);
    // Failed uploads leave no job directories behind.
    assert_eq!(
        std::fs::read_dir(data.path().join("jobs")).unwrap().count(),
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn token_auth() {
    let data = tempfile::tempdir().unwrap();
    let app = app(&data, Some("s3cret")).await;
    assert_eq!(get(&app, "/api/health").await.0, StatusCode::OK);
    assert_eq!(get(&app, "/api/jobs").await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        get(&app, "/api/jobs?token=wrong").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(get(&app, "/api/jobs?token=s3cret").await.0, StatusCode::OK);
    let req = Request::get("/api/jobs")
        .header(header::AUTHORIZATION, "Bearer s3cret")
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&app, req).await.0, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_queued_job() {
    let data = tempfile::tempdir().unwrap();
    let app = app(&data, None).await;
    // A heavy job keeps the single worker busy while the second one waits in the queue.
    let heavy = json!({ "mode": "corrected", "params": { "min_spacing": 1.0, "rays": 2000, "samples": 64 } });
    let (_, first) = post_json(&app, "/api/samples/1AHW_l_u/jobs", heavy).await;
    let (_, second) = post_json(&app, "/api/samples/1AHW_l_u/jobs", json!({})).await;
    let (first, second) = (
        first["id"].as_str().unwrap(),
        second["id"].as_str().unwrap(),
    );

    for id in [second, first] {
        let (s, _, _) = send(
            &app,
            Request::post(format!("/api/jobs/{id}/cancel"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(s, StatusCode::ACCEPTED);
    }
    for id in [first, second] {
        let start = Instant::now();
        loop {
            let job = get_json(&app, &format!("/api/jobs/{id}")).await;
            if job["status"] == "cancelled" {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(30), "{job}");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    // Cancelled jobs keep no outputs.
    assert!(!data.path().join("jobs").join(first).join("output").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn pdb_upload_generates_the_surface() {
    let data = tempfile::tempdir().unwrap();
    let app = app(&data, None).await;
    let pdb = std::fs::read(repo().join("data/complexes/1AHW/1AHW_l_u.pdb")).unwrap();
    let upload = |parts: Vec<(&'static str, Option<&'static str>, Vec<u8>)>| {
        let (ct, body) = multipart(&parts);
        Request::post("/api/jobs")
            .header(header::CONTENT_TYPE, ct)
            .body(Body::from(body))
            .unwrap()
    };

    // Legacy mode cannot start from atoms.
    let (s, _, _) = send(
        &app,
        upload(vec![
            ("pdb", Some("tf.pdb"), pdb.clone()),
            ("mode", None, b"legacy".to_vec()),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (s, b, _) = send(
        &app,
        upload(vec![
            ("pdb", Some("tissue_factor.pdb"), pdb),
            ("surface", None, br#"{"grid_spacing": 0.7}"#.to_vec()),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let job: Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(job["name"], "tissue_factor");
    let job = wait_done(&app, job["id"].as_str().unwrap()).await;
    let summary = &job["report"]["summary"];
    assert_eq!(summary["input"]["atoms"], 1622);
    assert_eq!(summary["surface"]["grid_spacing"], 0.7);
    assert!(summary["context_shapes"].as_u64().unwrap() > 0);
    let files: Vec<&str> = job["report"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert!(files.contains(&"tissue_factor.vert") && files.contains(&"tissue_factor.face"));
}

/// Fast docking settings: same pipeline, far fewer candidates (the full run takes ~10 s in
/// release; tests run unoptimised).
fn quick_dock_params() -> Value {
    json!({
        "descriptor": { "spacing": 4.0, "ring_radii": [2.0, 4.0, 6.0], "heights": [-2.0, 2.0],
                        "tilt_angles": [], "tilt_azimuths": 1 },
        "per_point": 5, "max_candidates": 800, "quick_top": 30, "refine_top": 8, "n_poses": 5
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn docking_job_end_to_end() {
    let data = tempfile::tempdir().unwrap();
    let app = app(&data, None).await;
    let complexes = get_json(&app, "/api/complexes").await;
    assert_eq!(complexes[0]["name"], "1AHW");
    assert_eq!(complexes[0]["has_native"], true);

    let (s, job) = post_json(
        &app,
        "/api/complexes/1AHW/dock",
        json!({ "params": quick_dock_params(), "complexes": 2 }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{job}");
    assert_eq!(job["mode"], "dock");
    let id = job["id"].as_str().unwrap().to_string();
    let job = wait_done(&app, &id).await;
    let summary = &job["report"]["summary"];
    assert_eq!(summary["kind"], "dock");
    let poses = summary["poses"].as_array().unwrap();
    assert!(!poses.is_empty() && poses.len() <= 5);
    // Every pose is scored against the bound complex.
    for p in poses {
        assert!(p["metrics"]["dockq"].as_f64().unwrap() >= 0.0);
        assert!(p["transform"]["rot"].is_array());
    }
    assert!(summary["native"]["pose"]["trans"].is_array());

    let (s, blob) = get(&app, &format!("/api/jobs/{id}/files/dock_viewer.bin")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(&blob[..4], b"DCK1");
    let files: Vec<&str> = job["report"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(files.iter().filter(|f| f.ends_with(".pdb")).count(), 2);
    let (s, pdb) = get(&app, &format!("/api/jobs/{id}/files/1AHW_pose_001.pdb")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(String::from_utf8_lossy(&pdb).contains("ATOM"));
}

#[tokio::test(flavor = "multi_thread")]
async fn docking_upload_validation() {
    let data = tempfile::tempdir().unwrap();
    let app = app(&data, None).await;
    let dir = repo().join("data/complexes/1AHW");
    let r = std::fs::read(dir.join("1AHW_r_u.pdb")).unwrap();
    let rb = std::fs::read(dir.join("1AHW_r_b.pdb")).unwrap();
    let upload = |parts: Vec<(&'static str, Option<&'static str>, Vec<u8>)>| {
        let (ct, body) = multipart(&parts);
        Request::post("/api/dock")
            .header(header::CONTENT_TYPE, ct)
            .body(Body::from(body))
            .unwrap()
    };
    // Missing ligand.
    let (s, _, _) = send(&app, upload(vec![("receptor", Some("r.pdb"), r.clone())])).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    // Native receptor without native ligand.
    let (s, _, _) = send(
        &app,
        upload(vec![
            ("receptor", Some("r.pdb"), r.clone()),
            ("ligand", Some("l.pdb"), r.clone()),
            ("native_receptor", Some("rb.pdb"), rb),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    // Unknown surface kind.
    let (s, _, _) = send(
        &app,
        upload(vec![
            ("receptor", Some("r.pdb"), r.clone()),
            ("ligand", Some("l.pdb"), r),
            ("surface", None, b"msms".to_vec()),
        ]),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(
        std::fs::read_dir(data.path().join("jobs"))
            .map(|d| d.count())
            .unwrap_or(0),
        0
    );
}
