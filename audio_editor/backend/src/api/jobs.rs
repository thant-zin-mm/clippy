//! Job endpoints (arch §6): SSE progress stream, status recovery, cancel.
//!
//! - `GET /api/jobs/{id}/events` — SSE `progress`/`done`/`error` stream.
//! - `GET /api/jobs/{id}` — status recovery after an SSE drop.
//! - `DELETE /api/jobs/{id}` — best-effort cancel of a queued/running job.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures_util::stream::Stream;
use serde::Serialize;

use crate::api::Shared;
use crate::error::ApiError;
use crate::jobs::JobEvent;

/// Router subtree for job routes (nested under `/api/jobs`).
pub fn routes() -> axum::Router<Shared> {
    axum::Router::new()
        .route("/{id}", axum::routing::get(status).delete(cancel))
        .route("/{id}/events", axum::routing::get(events))
}

/// Wire shape for `GET /api/jobs/{id}` (arch §6 recovery endpoint).
#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub state: crate::jobs::JobState,
    pub progress_pct: u8,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<crate::audio::prepare::PrepareResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::jobs::JobErrorPayload>,
}

/// `GET /api/jobs/{id}` — reconcile a dropped SSE stream.
pub async fn status(
    State((_state, _config, jobs)): State<Shared>,
    Path(id): Path<String>,
) -> Result<Json<JobStatusResponse>, ApiError> {
    let snap = jobs.snapshot(&id)?;
    Ok(Json(JobStatusResponse {
        job_id: snap.job_id,
        state: snap.state,
        progress_pct: snap.progress_pct,
        stage: snap.stage,
        result: snap.result,
        error: snap.error,
    }))
}

/// `DELETE /api/jobs/{id}` — best-effort cancel (FR-22, S).
pub async fn cancel(
    State((_state, _config, jobs)): State<Shared>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let snap = jobs.cancel(&id)?;
    if snap.state == crate::jobs::JobState::Cancelled {
        Ok(StatusCode::NO_CONTENT)
    } else {
        // Already running/finished — nothing to cancel, but not an error.
        Ok(StatusCode::NO_CONTENT)
    }
}

fn sse_event(job_event: JobEvent) -> Event {
    match job_event {
        JobEvent::Progress { pct, stage } => Event::default()
            .event("progress")
            .data(serde_json::json!({ "pct": pct, "stage": stage }).to_string()),
        JobEvent::Done(result) => Event::default()
            .event("done")
            .data(serde_json::to_string(&result).unwrap_or_default()),
        JobEvent::Error(err) => Event::default()
            .event("error")
            .data(serde_json::to_string(&err).unwrap_or_default()),
    }
}

/// `GET /api/jobs/{id}/events` — SSE progress stream (FR-22).
///
/// Ordering: subscribe first (so nothing sent after this point is lost), then
/// read the snapshot. Anything the worker broadcast in between is drained and
/// emitted first; if the job already finished, its terminal event is replayed
/// so a tab that (re)opens the stream after completion still learns the outcome
/// without polling (arch §6).
pub async fn events(
    State((_state, _config, jobs)): State<Shared>,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let mut rx = jobs.subscribe(&id)?;
    let snap = jobs.snapshot(&id)?;

    // Drain events the worker already broadcast (it may have raced us).
    let mut primed: Vec<JobEvent> = Vec::new();
    let mut saw_terminal = false;
    use tokio::sync::broadcast::error::TryRecvError;
    loop {
        match rx.try_recv() {
            Ok(event) => {
                saw_terminal |= matches!(event, JobEvent::Done(_) | JobEvent::Error(_));
                let stop = saw_terminal;
                primed.push(event);
                if stop {
                    break;
                }
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Lagged(_)) => continue,
            Err(TryRecvError::Closed) => {
                saw_terminal = true;
                break;
            }
        }
    }

    if !saw_terminal {
        if let Some(terminal) = terminal_event(&snap) {
            primed.push(terminal);
            saw_terminal = true;
        } else if primed.is_empty() {
            // First connection on an in-flight job: send the current position
            // so every subscriber starts with at least one event.
            primed.push(JobEvent::Progress {
                pct: snap.progress_pct,
                stage: snap.stage.clone(),
            });
        }
    }

    let follow_live = !saw_terminal;
    let stream = async_stream::stream! {
        for event in primed {
            yield Ok(sse_event(event));
        }
        if follow_live {
            loop {
                match rx.recv().await {
                    Ok(event) => yield Ok(sse_event(event)),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Client will reconcile via GET /api/jobs/{id}.
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}

/// Map a snapshot's terminal state onto the SSE event vocabulary.
fn terminal_event(snap: &crate::jobs::JobSnapshot) -> Option<JobEvent> {
    use crate::jobs::JobState;
    match snap.state {
        JobState::Done => snap.result.clone().map(JobEvent::Done),
        JobState::Error => snap.error.clone().map(JobEvent::Error),
        JobState::Cancelled => Some(JobEvent::Error(snap.error.clone().unwrap_or(
            crate::jobs::JobErrorPayload {
                code: "E_JOB_CANCELLED".into(),
                message: "job cancelled before it started".into(),
                hint: "Re-run the action to start a new job.".into(),
            },
        ))),
        JobState::Queued | JobState::Running => None,
    }
}
