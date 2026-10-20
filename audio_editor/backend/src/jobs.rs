//! Background job manager (arch §3.1, FR-22).
//!
//! One global worker processes a serial queue: `POST /prepare` enqueues a job,
//! the worker runs it off the async runtime, and clients stream
//! `progress`/`done`/`error` over `GET /api/jobs/{id}/events` (SSE). A
//! `GET /api/jobs/{id}` recovery endpoint returns the current state so a tab
//! that dropped its SSE stream can reconcile (arch §6).
//!
//! Panic isolation: the worker catches panics per job and turns them into a
//! job error rather than killing the server (NFR-4).

use std::collections::{HashMap, VecDeque};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::Serialize;
use tokio::sync::broadcast;

use crate::audio::prepare::{prepare_file, PrepareResult};
use crate::error::ApiError;
use crate::state::AppState;

/// Terminal + in-flight job states (arch §6 recovery endpoint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Running,
    Done,
    Error,
    Cancelled,
}

/// A single job's observable state (wire shape matches arch §6).
#[derive(Debug, Clone, Serialize)]
pub struct JobSnapshot {
    pub job_id: String,
    pub kind: String,
    pub state: JobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<PrepareResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JobErrorPayload>,
    pub progress_pct: u8,
    pub stage: String,
}

/// Serializable job failure (mirrors the FR-21 `{code,message,hint}` shape).
#[derive(Debug, Clone, Serialize)]
pub struct JobErrorPayload {
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl From<&ApiError> for JobErrorPayload {
    fn from(err: &ApiError) -> Self {
        Self {
            code: err.code().to_owned(),
            message: err.to_string(),
            hint: err.hint().to_owned(),
        }
    }
}

/// SSE event emitted on `GET /api/jobs/{id}/events` (arch §6).
#[derive(Debug, Clone)]
pub enum JobEvent {
    Progress { pct: u8, stage: String },
    Done(PrepareResult),
    Error(JobErrorPayload),
}

#[derive(Debug)]
struct JobEntry {
    snapshot: JobSnapshot,
    /// Which source this prepare job targets (worker-only; not serialized).
    source_id: String,
    broadcast: broadcast::Sender<JobEvent>,
}

#[derive(Debug, Default)]
struct JobsInner {
    jobs: HashMap<String, JobEntry>,
    queue: VecDeque<String>,
    running: Option<String>,
}

/// Handle to the job manager, cloned into axum state.
#[derive(Clone)]
pub struct JobManager {
    state: AppState,
    inner: Arc<Mutex<JobsInner>>,
    next_id: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
}

impl JobManager {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            inner: Arc::new(Mutex::new(JobsInner::default())),
            next_id: Arc::new(AtomicU64::new(1)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    fn lock_inner(&self) -> MutexGuard<'_, JobsInner> {
        self.inner.lock().expect("jobs mutex poisoned")
    }

    fn alloc_id(&self) -> String {
        let n = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("job_{n:08x}")
    }

    /// Enqueue a prepare job for `source_id`; returns the job id.
    pub fn enqueue_prepare(&self, source_id: &str) -> Result<String, ApiError> {
        let job_id = self.alloc_id();
        let (tx, _rx) = broadcast::channel(64);
        let snapshot = JobSnapshot {
            job_id: job_id.clone(),
            kind: "prepare".into(),
            state: JobState::Queued,
            result: None,
            error: None,
            progress_pct: 0,
            stage: "queued".into(),
        };
        {
            let mut inner = self.lock_inner();
            inner.jobs.insert(
                job_id.clone(),
                JobEntry {
                    snapshot,
                    source_id: source_id.to_owned(),
                    broadcast: tx,
                },
            );
            inner.queue.push_back(job_id.clone());
        }
        tracing::info!(job_id, source_id, "prepare job enqueued");
        Ok(job_id)
    }

    /// Current snapshot for a job, or `E_JOB_NOT_FOUND`.
    pub fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ApiError> {
        let inner = self.lock_inner();
        inner
            .jobs
            .get(job_id)
            .map(|e| e.snapshot.clone())
            .ok_or_else(|| ApiError::JobNotFound(job_id.to_owned()))
    }

    /// Subscribe to a job's event stream. Replays nothing (clients reconcile
    /// via `snapshot()` on connect), but delivers live + terminal events.
    pub fn subscribe(&self, job_id: &str) -> Result<broadcast::Receiver<JobEvent>, ApiError> {
        let inner = self.lock_inner();
        inner
            .jobs
            .get(job_id)
            .map(|e| e.broadcast.subscribe())
            .ok_or_else(|| ApiError::JobNotFound(job_id.to_owned()))
    }

    /// Best-effort cancel of a queued (not yet running) job (FR-22, S).
    pub fn cancel(&self, job_id: &str) -> Result<JobSnapshot, ApiError> {
        let mut inner = self.lock_inner();
        let was_queued = {
            let entry = inner
                .jobs
                .get_mut(job_id)
                .ok_or_else(|| ApiError::JobNotFound(job_id.to_owned()))?;
            if entry.snapshot.state == JobState::Queued {
                entry.snapshot.state = JobState::Cancelled;
                entry.snapshot.stage = "cancelled".into();
                true
            } else {
                false
            }
        };
        if was_queued {
            inner.queue.retain(|id| id != job_id);
            if let Some(entry) = inner.jobs.get(job_id) {
                let _ = entry.broadcast.send(JobEvent::Error(JobErrorPayload {
                    code: "E_JOB_CANCELLED".into(),
                    message: "job cancelled before it started".into(),
                    hint: "Re-run the action to start a new job.".into(),
                }));
            }
        }
        Ok(inner
            .jobs
            .get(job_id)
            .expect("checked above")
            .snapshot
            .clone())
    }

    fn report_progress(&self, job_id: &str, pct: u8, stage: &str) {
        let mut inner = self.lock_inner();
        if let Some(entry) = inner.jobs.get_mut(job_id) {
            if entry.snapshot.state == JobState::Running {
                entry.snapshot.progress_pct = pct;
                entry.snapshot.stage = stage.to_owned();
                let _ = entry.broadcast.send(JobEvent::Progress {
                    pct,
                    stage: stage.to_owned(),
                });
            }
        }
    }

    fn finish_ok(&self, job_id: &str, result: PrepareResult) {
        let mut inner = self.lock_inner();
        if let Some(entry) = inner.jobs.get_mut(job_id) {
            entry.snapshot.state = JobState::Done;
            entry.snapshot.progress_pct = 100;
            entry.snapshot.stage = "done".into();
            entry.snapshot.result = Some(result.clone());
            // Close the determinate bar before the typed payload (FR-22).
            let _ = entry.broadcast.send(JobEvent::Progress {
                pct: 100,
                stage: "done".into(),
            });
            let _ = entry.broadcast.send(JobEvent::Done(result));
        }
    }

    fn finish_err(&self, job_id: &str, err: ApiError) {
        let payload = JobErrorPayload::from(&err);
        let mut inner = self.lock_inner();
        if let Some(entry) = inner.jobs.get_mut(job_id) {
            entry.snapshot.state = JobState::Error;
            entry.snapshot.stage = "error".into();
            entry.snapshot.error = Some(payload.clone());
            let _ = entry.broadcast.send(JobEvent::Error(payload));
        }
    }

    /// Pop the next queued job, mark it running, and return `(job_id, source_id)`.
    fn begin_next(&self) -> Option<(String, String)> {
        let mut inner = self.lock_inner();
        let job_id = inner.queue.pop_front()?;
        let source_id = inner.jobs.get(&job_id)?.source_id.clone();
        if let Some(entry) = inner.jobs.get_mut(&job_id) {
            entry.snapshot.state = JobState::Running;
            entry.snapshot.stage = "starting".into();
        }
        inner.running = Some(job_id.clone());
        Some((job_id, source_id))
    }

    fn finish_running(&self) {
        let mut inner = self.lock_inner();
        inner.running = None;
    }

    /// Spawn the single global worker that drains the queue.
    pub fn spawn_worker(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        std::thread::spawn(move || {
            while !manager.shutdown.load(Ordering::Relaxed) {
                let Some((job_id, source_id)) = manager.begin_next() else {
                    std::thread::sleep(Duration::from_millis(25));
                    continue;
                };
                manager.run_prepare(&job_id, &source_id);
                manager.finish_running();
            }
        });
    }

    fn run_prepare(&self, job_id: &str, source_id: &str) {
        self.report_progress(job_id, 5, "decoding");
        let state = self.state.clone();
        let job_id_owned = job_id.to_owned();
        let source_id_owned = source_id.to_owned();

        // Panic isolation: a panicking prepare becomes a job error, not a
        // server crash (NFR-4).
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let (path, filename) = {
                let mut reg = state.lock();
                let rec = reg
                    .get_mut(&source_id_owned)
                    .ok_or_else(|| ApiError::SourceNotFound(source_id_owned.clone()))?;
                if rec.prepared.is_some() {
                    return Err(ApiError::AlreadyPrepared);
                }
                (rec.temp_path.clone(), rec.filename.clone())
            };
            self.report_progress(&job_id_owned, 20, "analyzing");
            let (result, prepared) = prepare_file(&source_id_owned, &filename, &path)?;
            {
                let mut reg = state.lock();
                if let Some(rec) = reg.get_mut(&source_id_owned) {
                    rec.prepared = Some(prepared);
                    rec.last_touched = std::time::Instant::now();
                }
            }
            Ok(result)
        }));

        match outcome {
            Ok(Ok(result)) => {
                self.report_progress(job_id, 90, "finalizing");
                self.finish_ok(job_id, result);
            }
            Ok(Err(err)) => self.finish_err(job_id, err),
            Err(_panic) => {
                tracing::error!(job_id, "prepare job panicked");
                self.finish_err(
                    job_id,
                    ApiError::PrepareFailed("internal error while preparing audio".into()),
                );
            }
        }
    }

    /// Stop the worker after the current job (best-effort shutdown hook).
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_creates_queued_snapshot() {
        let mgr = JobManager::new(AppState::new());
        let job_id = mgr.enqueue_prepare("src_x").unwrap();
        let snap = mgr.snapshot(&job_id).unwrap();
        assert_eq!(snap.state, JobState::Queued);
        assert_eq!(snap.kind, "prepare");
        assert_eq!(snap.progress_pct, 0);
        assert!(snap.result.is_none());
    }

    #[test]
    fn snapshot_unknown_job_is_not_found() {
        let mgr = JobManager::new(AppState::new());
        assert!(matches!(
            mgr.snapshot("job_dead"),
            Err(ApiError::JobNotFound(_))
        ));
        assert!(matches!(
            mgr.subscribe("job_dead"),
            Err(ApiError::JobNotFound(_))
        ));
        assert!(matches!(
            mgr.cancel("job_dead"),
            Err(ApiError::JobNotFound(_))
        ));
    }

    #[test]
    fn cancel_removes_queued_job_from_queue() {
        let mgr = JobManager::new(AppState::new());
        let job_id = mgr.enqueue_prepare("src_x").unwrap();
        let snap = mgr.cancel(&job_id).unwrap();
        assert_eq!(snap.state, JobState::Cancelled);
        // Second snapshot reflects the cancelled terminal state.
        assert_eq!(mgr.snapshot(&job_id).unwrap().state, JobState::Cancelled);
    }

    #[test]
    fn cancel_on_running_or_done_is_noop() {
        let mgr = JobManager::new(AppState::new());
        let job_id = mgr.enqueue_prepare("src_x").unwrap();
        // Force it into Running to prove cancel won't touch non-queued jobs.
        {
            let mut inner = mgr.lock_inner();
            let entry = inner.jobs.get_mut(&job_id).unwrap();
            entry.snapshot.state = JobState::Running;
        }
        let snap = mgr.cancel(&job_id).unwrap();
        assert_eq!(snap.state, JobState::Running);
    }

    #[test]
    fn subscribe_receives_broadcast_channel() {
        let mgr = JobManager::new(AppState::new());
        let job_id = mgr.enqueue_prepare("src_x").unwrap();
        let rx = mgr.subscribe(&job_id).unwrap();
        assert!(!rx.is_closed(), "subscriber must see a live channel");
    }

    #[test]
    fn job_error_payload_from_api_error() {
        let err = ApiError::SourceNotFound("src_1".into());
        let payload = JobErrorPayload::from(&err);
        assert_eq!(payload.code, "E_SOURCE_NOT_FOUND");
        assert!(!payload.hint.is_empty());
    }
}
