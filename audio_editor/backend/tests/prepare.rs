//! Integration tests for the P1-T1 upload → prepare → peaks → delete flow,
//! the job SSE stream, error paths, and the idle sweep (plan.md P1-T1).
//!
//! Harness: a real server is spawned on an ephemeral port, but the job worker
//! is started explicitly by each test *after* it has subscribed to the SSE
//! stream. That makes the full `progress … → done` sequence deterministic —
//! no events can be lost to a fast job racing the subscription.

use std::sync::Arc;
use std::time::Duration;

use silence_cutter::api;
use silence_cutter::config::Config;
use silence_cutter::jobs::JobManager;
use silence_cutter::state::AppState;

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// App under test: server running, worker NOT yet spawned.
struct TestApp {
    base: String,
    state: AppState,
    jobs: JobManager,
}

impl TestApp {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");

        let state = AppState::new();
        let jobs = JobManager::new(state.clone());
        let router = api::build_router_with_state(
            state.clone(),
            Config::parse(None, None, None, None),
            jobs.clone(),
        );

        tokio::spawn(async move {
            axum::serve(listener, router).await.expect("serve");
        });

        Self {
            base: format!("http://{addr}"),
            state,
            jobs,
        }
    }

    /// Start the global serial worker (call after subscribing to SSE).
    fn start_worker(&self) {
        Arc::new(self.jobs.clone()).spawn_worker();
    }

    async fn upload_bytes(
        &self,
        client: &reqwest::Client,
        filename: &str,
        bytes: Vec<u8>,
    ) -> serde_json::Value {
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .expect("mime");
        let form = reqwest::multipart::Form::new().part("file", part);
        let res = client
            .post(format!("{}/api/sources", self.base))
            .multipart(form)
            .send()
            .await
            .expect("upload");
        assert_eq!(res.status(), reqwest::StatusCode::OK, "upload status");
        res.json().await.expect("upload json")
    }

    async fn upload_fixture(&self, client: &reqwest::Client, name: &str) -> serde_json::Value {
        let bytes = std::fs::read(fixture_path(name)).expect("read fixture");
        self.upload_bytes(client, name, bytes).await
    }

    async fn prepare(&self, client: &reqwest::Client, source_id: &str) -> String {
        let res = client
            .post(format!("{}/api/sources/{}/prepare", self.base, source_id))
            .send()
            .await
            .expect("prepare");
        assert_eq!(res.status(), reqwest::StatusCode::OK, "prepare status");
        let job: serde_json::Value = res.json().await.expect("prepare json");
        job["job_id"].as_str().expect("job_id").to_string()
    }

    async fn snapshot(&self, client: &reqwest::Client, job_id: &str) -> serde_json::Value {
        client
            .get(format!("{}/api/jobs/{}", self.base, job_id))
            .send()
            .await
            .expect("snapshot")
            .json()
            .await
            .expect("snapshot json")
    }

    /// Poll the recovery endpoint until the job reaches a terminal state.
    async fn wait_terminal(&self, client: &reqwest::Client, job_id: &str) -> serde_json::Value {
        for _ in 0..200 {
            let snap = self.snapshot(client, job_id).await;
            let state = snap["state"].as_str().unwrap_or_default();
            if state == "done" || state == "error" || state == "cancelled" {
                return snap;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("job {job_id} did not reach a terminal state");
    }

    /// Open the SSE stream and return it (subscription is live once this
    /// resolves: axum builds the response after `subscribe` runs).
    async fn open_events(&self, client: &reqwest::Client, job_id: &str) -> reqwest::Response {
        client
            .get(format!("{}/api/jobs/{}/events", self.base, job_id))
            .send()
            .await
            .expect("sse open")
    }
}

/// Read an SSE byte stream until a terminal (`done`/`error`) event arrives.
/// Returns parsed events as `(event_name, data_json)`.
async fn read_events(stream: &mut reqwest::Response) -> Vec<(String, serde_json::Value)> {
    let mut events: Vec<(String, serde_json::Value)> = Vec::new();
    let mut buf = String::new();
    let mut event_name = String::new();
    let mut data_lines: Vec<String> = Vec::new();

    loop {
        let Some(chunk) = stream.chunk().await.expect("sse chunk") else {
            break; // stream closed without a terminal event
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim_end_matches('\r').to_owned();
            buf.drain(..=pos);

            if line.is_empty() {
                if !data_lines.is_empty() || !event_name.is_empty() {
                    let data = data_lines.join("\n");
                    let name = if event_name.is_empty() {
                        "message".to_owned()
                    } else {
                        std::mem::take(&mut event_name)
                    };
                    let value = serde_json::from_str(&data).unwrap_or(serde_json::Value::Null);
                    let terminal = name == "done" || name == "error";
                    events.push((name, value));
                    data_lines.clear();
                    if terminal {
                        return events;
                    }
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("event: ") {
                event_name = rest.trim().to_owned();
            } else if let Some(rest) = line.strip_prefix("data: ") {
                data_lines.push(rest.trim_end().to_owned());
            }
            // Ignore keep-alive comments (`: ping`) and anything else.
        }
    }
    events
}

/// Assert the wire contract: progress pcts are non-decreasing and a typed
/// terminal event closes the stream (P1-T1 acceptance).
fn assert_progress_then_terminal(events: &[(String, serde_json::Value)]) {
    assert!(!events.is_empty(), "no SSE events received");
    let mut last_pct: i64 = -1;
    for (name, data) in events {
        match name.as_str() {
            "progress" => {
                let pct = data["pct"]
                    .as_i64()
                    .unwrap_or_else(|| panic!("progress missing pct: {data}"));
                assert!(
                    pct >= last_pct,
                    "progress went backwards: {last_pct} -> {pct}"
                );
                assert!(pct <= 100, "pct out of range: {pct}");
                last_pct = pct;
                assert!(data["stage"].is_string(), "progress missing stage: {data}");
            }
            "done" => {
                assert!(
                    data["duration_ms"].as_u64().unwrap_or(0) > 0,
                    "done missing duration_ms: {data}"
                );
                assert_eq!(last_pct, 100, "done must follow progress 100");
            }
            "error" => {
                assert!(data["code"].is_string(), "error missing code: {data}");
                assert!(data["hint"].is_string(), "error missing hint: {data}");
            }
            other => panic!("unknown SSE event {other}: {data}"),
        }
    }
    let last = events.last().expect("checked non-empty").0.as_str();
    assert!(last == "done" || last == "error", "stream ended on {last}");
}

#[tokio::test]
async fn upload_prepare_peaks_delete_wav_happy_path() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();

    let uploaded = app.upload_fixture(&client, "short.wav").await;
    let source_id = uploaded["source_id"]
        .as_str()
        .expect("source_id")
        .to_string();

    let temp_path = app
        .state
        .lock()
        .get(&source_id)
        .map(|rec| rec.temp_path.clone())
        .expect("source registered");
    assert!(temp_path.exists(), "temp file must exist after upload");

    let job_id = app.prepare(&client, &source_id).await;
    let mut stream = app.open_events(&client, &job_id).await;
    app.start_worker();

    let events = read_events(&mut stream).await;
    assert_progress_then_terminal(&events);
    assert!(
        events.iter().any(|(name, _)| name == "progress"),
        "expected progress events, got {events:?}"
    );

    let terminal = app.wait_terminal(&client, &job_id).await;
    assert_eq!(terminal["state"], "done", "terminal: {terminal}");
    let result = &terminal["result"];
    assert!(result["duration_ms"].as_u64().unwrap_or(0) > 0);
    assert_eq!(result["sample_rate"].as_u64(), Some(44_100));
    assert_eq!(result["format"].as_str(), Some("wav"));
    assert!(result["noise_floor_db"].is_number());
    assert!(result["suggested_threshold_db"].is_number());

    // Peaks: exactly `columns` little-endian i16 min/max pairs (4 bytes each).
    let peaks = client
        .get(format!(
            "{}/api/sources/{}/peaks?start_sample=0&end_sample=1000&columns=8",
            app.base, source_id
        ))
        .send()
        .await
        .expect("peaks");
    assert_eq!(peaks.status(), reqwest::StatusCode::OK);
    let ct = peaks
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    assert_eq!(ct.as_deref(), Some("application/octet-stream"));
    let bytes = peaks.bytes().await.expect("peaks bytes");
    assert_eq!(bytes.len(), 8 * 4, "8 pairs × 4 bytes");

    // Peaks before prepare (conflict) is covered by a separate test.

    let del = client
        .delete(format!("{}/api/sources/{}", app.base, source_id))
        .send()
        .await
        .expect("delete");
    assert_eq!(del.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(!temp_path.exists(), "temp file must be gone after DELETE");

    let gone = client
        .get(format!(
            "{}/api/sources/{}/peaks?columns=8",
            app.base, source_id
        ))
        .send()
        .await
        .expect("peaks after delete");
    assert_eq!(gone.status(), reqwest::StatusCode::NOT_FOUND);
    let body: serde_json::Value = gone.json().await.expect("json");
    assert_eq!(body["code"], "E_SOURCE_NOT_FOUND");
}

#[tokio::test]
async fn prepare_mp3_happy_path() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();

    let uploaded = app.upload_fixture(&client, "short.mp3").await;
    let source_id = uploaded["source_id"].as_str().unwrap().to_string();

    let job_id = app.prepare(&client, &source_id).await;
    let mut stream = app.open_events(&client, &job_id).await;
    app.start_worker();

    let events = read_events(&mut stream).await;
    assert_progress_then_terminal(&events);

    let terminal = app.wait_terminal(&client, &job_id).await;
    assert_eq!(terminal["state"], "done", "terminal: {terminal}");
    assert_eq!(terminal["result"]["format"].as_str(), Some("mp3"));
    assert_eq!(terminal["result"]["sample_rate"].as_u64(), Some(44_100));
}

#[tokio::test]
async fn prepare_unknown_source_is_not_found() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/api/sources/src_missing/prepare", app.base))
        .send()
        .await
        .expect("prepare");
    assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);
    let body: serde_json::Value = res.json().await.expect("json");
    assert_eq!(body["code"], "E_SOURCE_NOT_FOUND");
    assert!(!body["hint"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn peaks_before_prepare_is_conflict() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();
    let uploaded = app.upload_fixture(&client, "short.wav").await;
    let source_id = uploaded["source_id"].as_str().unwrap().to_string();

    let res = client
        .get(format!(
            "{}/api/sources/{}/peaks?columns=8",
            app.base, source_id
        ))
        .send()
        .await
        .expect("peaks");
    assert_eq!(res.status(), reqwest::StatusCode::CONFLICT);
    let body: serde_json::Value = res.json().await.expect("json");
    assert_eq!(body["code"], "E_NOT_PREPARED");
}

#[tokio::test]
async fn prepare_twice_is_conflict() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();
    let uploaded = app.upload_fixture(&client, "short.wav").await;
    let source_id = uploaded["source_id"].as_str().unwrap().to_string();

    let job_id = app.prepare(&client, &source_id).await;
    let mut stream = app.open_events(&client, &job_id).await;
    app.start_worker();
    let _ = read_events(&mut stream).await;
    let terminal = app.wait_terminal(&client, &job_id).await;
    assert_eq!(terminal["state"], "done");

    let res = client
        .post(format!("{}/api/sources/{}/prepare", app.base, source_id))
        .send()
        .await
        .expect("second prepare");
    assert_eq!(res.status(), reqwest::StatusCode::CONFLICT);
    let body: serde_json::Value = res.json().await.expect("json");
    assert_eq!(body["code"], "E_ALREADY_PREPARED");
}

#[tokio::test]
async fn upload_corrupt_file_fails_prepare() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();

    // WAV magic with no fmt/data chunks — rejected by the decoder.
    let corrupt = b"RIFF\x24\x00\x00\x00WAVE".to_vec();
    let uploaded = app.upload_bytes(&client, "corrupt.wav", corrupt).await;
    let source_id = uploaded["source_id"].as_str().unwrap().to_string();

    let job_id = app.prepare(&client, &source_id).await;
    let mut stream = app.open_events(&client, &job_id).await;
    app.start_worker();

    let events = read_events(&mut stream).await;
    let last = events.last().expect("at least one event");
    assert_eq!(last.0, "error", "events: {events:?}");
    let code = last.1["code"].as_str().unwrap_or_default();
    assert!(
        code == "E_CORRUPT_FILE" || code == "E_UNSUPPORTED_FORMAT",
        "unexpected error code {code}"
    );
    assert!(last.1["hint"].is_string());

    let terminal = app.wait_terminal(&client, &job_id).await;
    assert_eq!(terminal["state"], "error");
}

#[tokio::test]
async fn terminal_event_is_replayed_after_completion() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();

    let uploaded = app.upload_fixture(&client, "short.wav").await;
    let source_id = uploaded["source_id"].as_str().unwrap().to_string();
    let job_id = app.prepare(&client, &source_id).await;
    app.start_worker();
    let terminal = app.wait_terminal(&client, &job_id).await;
    assert_eq!(terminal["state"], "done");

    // Reconnect after completion: the stream must replay `done` immediately
    // so a tab never sits on a silent EventSource (arch §6 reconciliation).
    let mut stream = app.open_events(&client, &job_id).await;
    let events = tokio::time::timeout(Duration::from_secs(5), read_events(&mut stream))
        .await
        .expect("replayed terminal event within 5s");
    assert_eq!(events.len(), 1, "events: {events:?}");
    assert_eq!(events[0].0, "done");
    assert!(events[0].1["duration_ms"].as_u64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn unknown_job_endpoints_are_not_found() {
    let app = TestApp::start().await;
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{}/api/jobs/job_nope", app.base))
        .send()
        .await
        .expect("status");
    assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);
    let body: serde_json::Value = res.json().await.expect("json");
    assert_eq!(body["code"], "E_JOB_NOT_FOUND");

    let res = client
        .get(format!("{}/api/jobs/job_nope/events", app.base))
        .send()
        .await
        .expect("events");
    assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);

    let res = client
        .delete(format!("{}/api/jobs/job_nope", app.base))
        .send()
        .await
        .expect("cancel");
    assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn idle_sweep_removes_stale_sources() {
    let state = AppState::new();
    let temp = silence_cutter::state::new_temp_path("src_stale_test");
    std::fs::write(&temp, b"stale").expect("write temp");
    state.insert("src_stale_test".into(), "stale.wav".into(), temp.clone());

    // Age the record (the registry sweeps on last_touched, not file mtime).
    {
        let mut reg = state.lock();
        reg.get_mut("src_stale_test").expect("record").last_touched -= Duration::from_secs(7200);
    }

    let removed = state.sweep_idle(Duration::from_secs(3600));
    assert!(
        removed.contains(&"src_stale_test".to_string()),
        "removed: {removed:?}"
    );
    assert!(!temp.exists(), "stale temp file must be deleted");
}
