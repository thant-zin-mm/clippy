//! Shared application state and temp-file lifecycle.
//!
//! The source registry (upload → temp file → derived caches) arrives in
//! P1-T1; this module owns the temp-file prefix and the startup sweep so
//! crash leftovers never accumulate (NFR-4, arch §3.1).
//!
//! Safety: the sweep runs only files **older than the configured TTL**, so a
//! concurrently running instance's fresh source files are never deleted.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const TEMP_FILE_PREFIX: &str = "silence-cutter-";

/// Shared axum state. Empty for now; P1-T1 adds the source registry.
#[derive(Debug, Default, Clone)]
pub struct AppState;

/// OS temp directory used for uploaded sources and exports.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
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
}
