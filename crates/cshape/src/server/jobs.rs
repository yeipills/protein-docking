//! Job queue: runs computations on blocking threads and broadcasts their progress.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cs_core::Error as CoreError;
use cs_core::progress::Progress;
use serde::Serialize;
use tokio::sync::{broadcast, mpsc};

use super::db::{Db, Status};
use crate::pipeline::{self, Inputs, Spec, SurfaceInput};

#[derive(Debug, Clone, Serialize)]
pub struct JobEvent {
    pub status: Status,
    pub progress: f32,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Live {
    cancel: Arc<AtomicBool>,
    events: broadcast::Sender<JobEvent>,
    last: Arc<Mutex<JobEvent>>,
}

pub struct Jobs {
    pub db: Arc<Db>,
    pub data_dir: PathBuf,
    live: Mutex<HashMap<String, Live>>,
    queue: mpsc::UnboundedSender<String>,
}

pub const INPUT_DIR: &str = "input";
pub const OUTPUT_DIR: &str = "output";
pub const INPUT_VERT: &str = "surface.vert";
pub const INPUT_FACE: &str = "surface.face";
pub const INPUT_STL: &str = "surface.stl";
pub const INPUT_PDB: &str = "structure.pdb";

impl Jobs {
    /// Creates the manager and starts `workers` worker tasks. Jobs left unfinished by a
    /// previous run are queued again.
    pub fn start(db: Arc<Db>, data_dir: PathBuf, workers: usize) -> anyhow::Result<Arc<Jobs>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let jobs = Arc::new(Jobs {
            db,
            data_dir,
            live: Mutex::new(HashMap::new()),
            queue: tx,
        });
        let rx = Arc::new(tokio::sync::Mutex::new(rx));
        for _ in 0..workers.max(1) {
            let (jobs, rx) = (jobs.clone(), rx.clone());
            tokio::spawn(async move {
                loop {
                    let next = rx.lock().await.recv().await;
                    let Some(id) = next else { break };
                    jobs.run(&id).await;
                }
            });
        }
        for id in jobs.db.unfinished()? {
            jobs.db.requeue(&id)?;
            jobs.enqueue(&id);
        }
        Ok(jobs)
    }

    pub fn job_dir(&self, id: &str) -> PathBuf {
        self.data_dir.join("jobs").join(id)
    }

    /// Registers a job whose input files are already in `job_dir(id)/input` and queues it.
    pub fn submit(&self, id: &str, spec: &Spec) -> anyhow::Result<()> {
        let spec_json = serde_json::to_value(spec)?;
        let mode = serde_json::to_value(spec.mode)?;
        self.db.insert(
            id,
            &spec.name,
            mode.as_str().unwrap_or_default(),
            &spec_json,
        )?;
        self.enqueue(id);
        Ok(())
    }

    fn enqueue(&self, id: &str) {
        let initial = JobEvent {
            status: Status::Queued,
            progress: 0.0,
            stage: String::new(),
            error: None,
        };
        let (events, _) = broadcast::channel(64);
        self.live.lock().unwrap().insert(
            id.to_string(),
            Live {
                cancel: Arc::new(AtomicBool::new(false)),
                events,
                last: Arc::new(Mutex::new(initial)),
            },
        );
        let _ = self.queue.send(id.to_string());
    }

    /// Current state and an event receiver, if the job is queued or running.
    pub fn subscribe(&self, id: &str) -> Option<(JobEvent, broadcast::Receiver<JobEvent>)> {
        let live = self.live.lock().unwrap();
        let l = live.get(id)?;
        let rx = l.events.subscribe();
        let last = l.last.lock().unwrap().clone();
        Some((last, rx))
    }

    /// Requests cancellation. Returns false if the job is not queued or running.
    /// A queued job is cancelled immediately; a running one stops at its next checkpoint.
    pub fn cancel(&self, id: &str) -> bool {
        let mut live = self.live.lock().unwrap();
        let Some(l) = live.get(id) else {
            return false;
        };
        l.cancel.store(true, Ordering::Relaxed);
        let queued = l.last.lock().unwrap().status == Status::Queued;
        if queued {
            let l = live.remove(id).unwrap();
            if let Err(e) = self.db.finish(id, Status::Cancelled, None, None) {
                tracing::error!(job = id, "could not record cancellation: {e:#}");
            }
            let _ = l.events.send(JobEvent {
                status: Status::Cancelled,
                progress: 0.0,
                stage: "cancelled".into(),
                error: None,
            });
        }
        true
    }

    async fn run(self: &Arc<Self>, id: &str) {
        let Some((cancel, events, last)) = self
            .live
            .lock()
            .unwrap()
            .get(id)
            .map(|l| (l.cancel.clone(), l.events.clone(), l.last.clone()))
        else {
            return;
        };
        let publish = {
            let (events, last) = (events.clone(), last.clone());
            move |e: JobEvent| {
                *last.lock().unwrap() = e.clone();
                let _ = events.send(e);
            }
        };

        let outcome = if cancel.load(Ordering::Relaxed) {
            Err(CoreError::Cancelled.into())
        } else {
            self.execute(id, cancel.clone(), publish.clone()).await
        };

        let (status, error, report) = match outcome {
            Ok(report) => (Status::Done, None, Some(report)),
            Err(e)
                if e.downcast_ref::<CoreError>()
                    .is_some_and(|e| matches!(e, CoreError::Cancelled)) =>
            {
                (Status::Cancelled, None, None)
            }
            Err(e) => (Status::Failed, Some(format!("{e:#}")), None),
        };
        if status != Status::Done {
            // Partial outputs are not useful.
            let _ = std::fs::remove_dir_all(self.job_dir(id).join(OUTPUT_DIR));
        }
        if let Err(e) = self
            .db
            .finish(id, status, error.as_deref(), report.as_ref())
        {
            tracing::error!(job = id, "could not record job result: {e:#}");
        }
        let progress = last.lock().unwrap().progress;
        publish(JobEvent {
            status,
            progress: if status == Status::Done {
                1.0
            } else {
                progress
            },
            stage: status.as_str().into(),
            error,
        });
        self.live.lock().unwrap().remove(id);
        tracing::info!(job = id, status = status.as_str(), "job finished");
    }

    async fn execute(
        self: &Arc<Self>,
        id: &str,
        cancel: Arc<AtomicBool>,
        publish: impl Fn(JobEvent) + Send + Sync + 'static,
    ) -> anyhow::Result<serde_json::Value> {
        let job = self
            .db
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("job vanished"))?;
        let spec: Spec = serde_json::from_value(job.spec.unwrap_or_default())?;
        self.db.set_running(id)?;
        publish(JobEvent {
            status: Status::Running,
            progress: 0.0,
            stage: "starting".into(),
            error: None,
        });
        tracing::info!(job = id, name = %spec.name, mode = ?spec.mode, "job started");

        let dir = self.job_dir(id);
        let jobs = self.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let input = dir.join(INPUT_DIR);
            let stl = input.join(INPUT_STL);
            let pdb = input.join(INPUT_PDB);
            let surface = if pdb.exists() {
                SurfaceInput::Pdb(pdb)
            } else {
                SurfaceInput::Msms {
                    vert: input.join(INPUT_VERT),
                    face: input.join(INPUT_FACE),
                }
            };
            let inputs = Inputs {
                surface,
                stl: stl.exists().then_some(stl),
            };
            let progress = JobProgress {
                id: &id,
                db: &jobs.db,
                cancel: &cancel,
                publish: &publish,
                throttle: Mutex::new((
                    Instant::now() - Duration::from_secs(1),
                    String::new(),
                    -1.0,
                )),
            };
            let report = pipeline::execute(&spec, &inputs, &dir.join(OUTPUT_DIR), &progress)?;
            Ok(serde_json::to_value(report)?)
        })
        .await?
    }

    /// Deletes a finished job's row and files. Returns false if it does not exist.
    pub fn delete(&self, id: &str) -> anyhow::Result<bool> {
        let existed = self.db.delete(id)?;
        let dir = self.job_dir(id);
        if dir.starts_with(&self.data_dir) && dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(existed)
    }
}

struct JobProgress<'a, P: Fn(JobEvent) + Sync> {
    id: &'a str,
    db: &'a Db,
    cancel: &'a AtomicBool,
    publish: &'a P,
    /// Last (time, stage, fraction) sent, to limit event and database traffic.
    throttle: Mutex<(Instant, String, f32)>,
}

impl<P: Fn(JobEvent) + Sync> Progress for JobProgress<'_, P> {
    fn report(&self, stage: &str, fraction: f32) {
        let mut t = self.throttle.lock().unwrap();
        let stage_changed = t.1 != stage;
        if !stage_changed && (t.0.elapsed() < Duration::from_millis(200) || fraction - t.2 < 0.005)
        {
            return;
        }
        *t = (Instant::now(), stage.to_string(), fraction);
        drop(t);
        let _ = self.db.set_progress(self.id, fraction, stage);
        (self.publish)(JobEvent {
            status: Status::Running,
            progress: fraction,
            stage: stage.to_string(),
            error: None,
        });
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}
