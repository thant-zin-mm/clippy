//! Shared application state and temp-file lifecycle (arch §3.1).
//!
//! Holds the source registry: `source_id → SourceRecord`. A source is created
//! on upload (temp file written), populated by `prepare` (peak pyramid + frame
//! arrays + metadata), and removed on `DELETE` or by the idle TTL sweep. The
//! startup sweep deletes leftover `silence-cutter-*` files from crashed runs;
//! the idle sweep covers sources whose tabs vanished without `DELETE`.
//!
//! Safety: sweeps only touch files **older than the configured TTL**, so a
//! concurrently running instance's fresh source files are never deleted.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use crate::audio::prepare::PreparedSource;
use crate::error::ApiError;

pub const TEMP_FILE_PREFIX: &str = "silence-cutter-";

/// Shared axum state: the source registry behind a mutex.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<Registry>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Registry::default())),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, Registry> {
        self.inner.lock().expect("registry mutex poisoned")
    }

    /// Register a freshly uploaded source; returns its id.
    pub fn insert(&self, source_id: String, filename: String, temp_path: PathBuf) {
        self.lock().insert(source_id, filename, temp_path);
    }

    /// Remove a source and delete its temp file from disk.
    pub fn remove(&self, id: &str) -> Option<SourceRecord> {
        self.lock().remove(id)
    }

    /// Idle TTL sweep; returns ids that were removed.
    pub fn sweep_idle(&self, ttl: Duration) -> Vec<String> {
        self.lock().sweep_idle(ttl)
    }
}

/// One uploaded source and everything derived from it.
#[derive(Debug)]
pub struct SourceRecord {
    pub source_id: String,
    pub filename: String,
    pub temp_path: PathBuf,
    pub created_at: Instant,
    pub last_touched: Instant,
    pub prepared: Option<PreparedSource>,
}

impl SourceRecord {
    fn new(source_id: String, filename: String, temp_path: PathBuf) -> Self {
        let now = Instant::now();
        Self {
            source_id,
            filename,
            temp_path,
            created_at: now,
            last_touched: now,
            prepared: None,
        }
    }
}

/// In-memory registry of sources (arch §3.1 `state`).
#[derive(Debug, Default)]
pub struct Registry {
    sources: HashMap<String, SourceRecord>,
}

impl Registry {
    /// Register a freshly uploaded source; returns its id.
    pub fn insert(&mut self, source_id: String, filename: String, temp_path: PathBuf) {
        let record = SourceRecord::new(source_id.clone(), filename, temp_path);
        self.sources.insert(source_id, record);
    }

    pub fn get(&self, id: &str) -> Option<&SourceRecord> {
        self.sources.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut SourceRecord> {
        self.sources.get_mut(id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.sources.contains_key(id)
    }

    /// Remove a source and delete its temp file from disk.
    ///
    /// Returns the removed record so callers can log/clean derived state.
    pub fn remove(&mut self, id: &str) -> Option<SourceRecord> {
        let record = self.sources.remove(id)?;
        if let Err(err) = fs::remove_file(&record.temp_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(source_id = id, error = %err, "failed to delete source temp file");
            }
        }
        Some(record)
    }

    /// Idle TTL sweep: drop sources untouched for longer than `ttl`.
    ///
    /// Returns the ids that were removed so the caller can log the count.
    pub fn sweep_idle(&mut self, ttl: Duration) -> Vec<String> {
        let now = Instant::now();
        let expired: Vec<String> = self
            .sources
            .values()
            .filter(|rec| now.duration_since(rec.last_touched) > ttl)
            .map(|rec| rec.source_id.clone())
            .collect();
        for id in &expired {
            self.remove(id);
        }
        expired
    }
}

/// OS temp directory used for uploaded sources and exports.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
}

/// Generate a unique, filesystem-safe temp file path for a source.
pub fn new_temp_path(source_id: &str) -> PathBuf {
    temp_dir().join(format!("{TEMP_FILE_PREFIX}{source_id}"))
}

/// Generate an opaque source id (no client input involved).
pub fn new_source_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let random = std::time::Instant::now().elapsed().as_nanos() as u64;
    format!("src_{nanos:016x}_{random:08x}")
}

/// Validate that a filename is safe to echo back (never used as a path).
pub fn sanitize_filename(name: &str) -> Result<String, ApiError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ApiError::InvalidParameter("filename is empty".into()));
    }
    if trimmed.len() > 255 {
        return Err(ApiError::InvalidParameter(
            "filename longer than 255 bytes".into(),
        ));
    }
    // Strip any path separators or control characters.
    let cleaned: String = trimmed
        .chars()
        .filter(|c| !c.is_control() && *c != '/' && *c != '\\')
        .collect();
    if cleaned.trim().is_empty() {
        return Err(ApiError::InvalidParameter("filename is empty".into()));
    }
    Ok(cleaned)
}

/// Remove leftover `silence-cutter-*` files older than `ttl` in the OS temp dir.
pub fn sweep_stale_temp_files(ttl: Duration) -> usize {
    sweep_in(&temp_dir(), ttl)
}

/// Sweep aged `TEMP_FILE_PREFIX*` regular files inside `dir`; files modified
/// within the last `ttl` are kept. Returns how many files were removed.
pub fn sweep_in(dir: &Path, ttl: Duration) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(TEMP_FILE_PREFIX) {
            continue;
        }
        let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
        if !is_file {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default();
        if age < ttl {
            continue; // possibly owned by a live instance — leave it alone
        }
        match fs::remove_file(entry.path()) {
            Ok(()) => {
                tracing::warn!(file = name, "removed stale temp file");
                removed += 1;
            }
            Err(err) => tracing::warn!(file = name, %err, "could not remove stale temp file"),
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("silence-cutter-sweep-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn backdate(path: &Path, age: Duration) {
        let file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    #[test]
    fn sweep_removes_only_aged_prefixed_files() {
        let dir = scratch_dir();
        let ours = dir.join(format!("{TEMP_FILE_PREFIX}leftover-1.tmp"));
        let foreign = dir.join("unrelated-file.txt");
        File::create(&ours).unwrap();
        File::create(&foreign).unwrap();
        backdate(&ours, Duration::from_secs(7200));
        backdate(&foreign, Duration::from_secs(7200));

        let removed = sweep_in(&dir, Duration::from_secs(3600));

        assert_eq!(removed, 1);
        assert!(!ours.exists(), "aged prefixed file should be removed");
        assert!(foreign.exists(), "non-prefixed file must survive");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sweep_keeps_fresh_files_from_live_instances() {
        let dir = scratch_dir();
        let fresh = dir.join(format!("{TEMP_FILE_PREFIX}in-use.tmp"));
        File::create(&fresh).unwrap(); // mtime = now

        let removed = sweep_in(&dir, Duration::from_secs(3600));

        assert_eq!(removed, 0);
        assert!(fresh.exists(), "fresh file must survive the sweep");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sweep_on_missing_dir_is_harmless() {
        let missing = std::env::temp_dir().join("silence-cutter-sweep-test-missing-dir");
        assert_eq!(sweep_in(&missing, Duration::from_secs(3600)), 0);
    }

    #[test]
    fn sanitize_filename_strips_paths_and_controls() {
        assert_eq!(sanitize_filename("song.mp3").unwrap(), "song.mp3");
        assert_eq!(sanitize_filename("a/b\\c.wav").unwrap(), "abc.wav");
        assert_eq!(sanitize_filename("ok name.flac").unwrap(), "ok name.flac");
        assert!(sanitize_filename("   ").is_err());
        assert!(sanitize_filename("").is_err());
        assert!(sanitize_filename(&"x".repeat(300)).is_err());
        assert!(sanitize_filename("bad\u{0}.mp3").is_ok());
    }

    #[test]
    fn source_ids_are_unique_and_prefixed() {
        let a = new_source_id();
        let b = new_source_id();
        assert_ne!(a, b);
        assert!(a.starts_with("src_"));
        assert!(b.starts_with("src_"));
    }

    #[test]
    fn registry_insert_get_remove_roundtrip() {
        let mut reg = Registry::default();
        let id = "src_test".to_string();
        let path = std::env::temp_dir().join(format!("{TEMP_FILE_PREFIX}registry-test"));
        File::create(&path).unwrap();

        reg.insert(id.clone(), "a.wav".into(), path.clone());
        assert!(reg.contains(&id));
        assert_eq!(reg.get(&id).unwrap().filename, "a.wav");
        assert!(!reg.get(&id).unwrap().prepared.is_some());

        let removed = reg.remove(&id).expect("record exists");
        assert_eq!(removed.source_id, id);
        assert!(!reg.contains(&id));
        assert!(!path.exists(), "temp file should be deleted on remove");
    }

    #[test]
    fn idle_sweep_removes_only_expired_sources() {
        let mut reg = Registry::default();
        let fresh_id = "src_fresh".to_string();
        let stale_id = "src_stale".to_string();
        let fresh_path = std::env::temp_dir().join(format!("{TEMP_FILE_PREFIX}sweep-fresh"));
        let stale_path = std::env::temp_dir().join(format!("{TEMP_FILE_PREFIX}sweep-stale"));
        File::create(&fresh_path).unwrap();
        File::create(&stale_path).unwrap();

        reg.insert(fresh_id.clone(), "fresh.wav".into(), fresh_path.clone());
        reg.insert(stale_id.clone(), "stale.wav".into(), stale_path.clone());
        // Age the stale record artificially.
        reg.get_mut(&stale_id).unwrap().last_touched -= Duration::from_secs(7200);

        let removed = reg.sweep_idle(Duration::from_secs(3600));

        assert_eq!(removed, vec![stale_id.clone()]);
        assert!(reg.contains(&fresh_id));
        assert!(!reg.contains(&stale_id));
        assert!(!stale_path.exists());
        assert!(fresh_path.exists());

        reg.remove(&fresh_id);
        fs::remove_file(&fresh_path).ok();
        fs::remove_file(&stale_path).ok();
    }
}
