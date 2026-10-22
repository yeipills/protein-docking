//! SQLite persistence for jobs.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Queued => "queued",
            Status::Running => "running",
            Status::Done => "done",
            Status::Failed => "failed",
            Status::Cancelled => "cancelled",
        }
    }

    fn parse(s: &str) -> Status {
        match s {
            "queued" => Status::Queued,
            "running" => Status::Running,
            "done" => Status::Done,
            "cancelled" => Status::Cancelled,
            _ => Status::Failed,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Status::Done | Status::Failed | Status::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub status: Status,
    pub progress: f32,
    pub stage: String,
    pub error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    /// Full job specification (only in single-job responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<serde_json::Value>,
    /// Pipeline report: summary, timings and output files (only in single-job responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<serde_json::Value>,
}

pub struct Db(Mutex<Connection>);

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS jobs (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    mode        TEXT NOT NULL,
    status      TEXT NOT NULL,
    progress    REAL NOT NULL DEFAULT 0,
    stage       TEXT NOT NULL DEFAULT '',
    error       TEXT,
    spec        TEXT NOT NULL,
    report      TEXT,
    created_at  TEXT NOT NULL,
    started_at  TEXT,
    finished_at TEXT
);
CREATE INDEX IF NOT EXISTS jobs_created ON jobs (created_at DESC);
";

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Db> {
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Db(Mutex::new(conn)))
    }

    pub fn open_in_memory() -> anyhow::Result<Db> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Db(Mutex::new(conn)))
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic while holding the lock leaves the connection usable; keep serving.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn insert(
        &self,
        id: &str,
        name: &str,
        mode: &str,
        spec: &serde_json::Value,
    ) -> anyhow::Result<()> {
        self.conn().execute(
            "INSERT INTO jobs (id, name, mode, status, spec, created_at) VALUES (?1, ?2, ?3, 'queued', ?4, ?5)",
            params![id, name, mode, spec.to_string(), now()],
        )?;
        Ok(())
    }

    pub fn list(&self) -> anyhow::Result<Vec<Job>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, mode, status, progress, stage, error, created_at, started_at, finished_at
             FROM jobs ORDER BY created_at DESC LIMIT 500",
        )?;
        let rows = stmt.query_map([], |r| row_to_job(r, false))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get(&self, id: &str) -> anyhow::Result<Option<Job>> {
        let conn = self.conn();
        let job = conn
            .query_row(
                "SELECT id, name, mode, status, progress, stage, error, created_at, started_at, finished_at,
                        spec, report
                 FROM jobs WHERE id = ?1",
                [id],
                |r| row_to_job(r, true),
            )
            .optional()?;
        Ok(job)
    }

    pub fn set_running(&self, id: &str) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE jobs SET status = 'running', started_at = ?2, progress = 0, stage = '' WHERE id = ?1",
            params![id, now()],
        )?;
        Ok(())
    }

    pub fn set_progress(&self, id: &str, progress: f32, stage: &str) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE jobs SET progress = ?2, stage = ?3 WHERE id = ?1",
            params![id, progress, stage],
        )?;
        Ok(())
    }

    pub fn finish(
        &self,
        id: &str,
        status: Status,
        error: Option<&str>,
        report: Option<&serde_json::Value>,
    ) -> anyhow::Result<()> {
        let progress = if status == Status::Done { 1.0 } else { -1.0 };
        self.conn().execute(
            "UPDATE jobs SET status = ?2, error = ?3, report = ?4, finished_at = ?5,
                    progress = CASE WHEN ?6 >= 0 THEN ?6 ELSE progress END
             WHERE id = ?1",
            params![
                id,
                status.as_str(),
                error,
                report.map(|r| r.to_string()),
                now(),
                progress
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> anyhow::Result<bool> {
        Ok(self
            .conn()
            .execute("DELETE FROM jobs WHERE id = ?1", [id])?
            > 0)
    }

    /// Jobs that were queued or running when the server last stopped, oldest first.
    pub fn unfinished(&self) -> anyhow::Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id FROM jobs WHERE status IN ('queued', 'running') ORDER BY created_at ASC",
        )?;
        let ids = stmt.query_map([], |r| r.get(0))?;
        Ok(ids.collect::<Result<_, _>>()?)
    }

    pub fn requeue(&self, id: &str) -> anyhow::Result<()> {
        self.conn().execute(
            "UPDATE jobs SET status = 'queued', progress = 0, stage = '', started_at = NULL WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }
}

fn row_to_job(r: &rusqlite::Row, full: bool) -> rusqlite::Result<Job> {
    let json = |s: Option<String>| s.and_then(|s| serde_json::from_str(&s).ok());
    Ok(Job {
        id: r.get(0)?,
        name: r.get(1)?,
        mode: r.get(2)?,
        status: Status::parse(&r.get::<_, String>(3)?),
        progress: r.get(4)?,
        stage: r.get(5)?,
        error: r.get(6)?,
        created_at: r.get(7)?,
        started_at: r.get(8)?,
        finished_at: r.get(9)?,
        spec: if full { json(r.get(10)?) } else { None },
        report: if full { json(r.get(11)?) } else { None },
    })
}
