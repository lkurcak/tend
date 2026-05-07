use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// === Timestamp Formatting ===

/// Format a `SystemTime` as `YYYY-MM-DDThh-mm-ss` (UTC) for use in filenames.
fn format_timestamp(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (year, month, day, hour, min, sec) = unix_to_civil(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}-{min:02}-{sec:02}")
}

/// Convert a unix timestamp (seconds since epoch) to (year, month, day, hour, minute, second) in UTC.
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
const fn unix_to_civil(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let sec = (secs % 60) as u32;
    let min = ((secs / 60) % 60) as u32;
    let hour = ((secs / 3600) % 24) as u32;

    // Days since epoch (1970-01-01)
    let days = (secs / 86400) as i64;

    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // year of era [0, 399]
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month index [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (if month <= 2 { y + 1 } else { y }) as u32;

    (year, month, day, hour, min, sec)
}

// === Metadata Entry ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntry {
    pub run_id: u64,
    /// Unix timestamp in seconds
    pub at: u64,
    pub kind: MetaEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetaEventKind {
    Started,
    Exited {
        status: ExitStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_secs: Option<u64>,
    },
    Restarting {
        reason: String,
        attempt: u64,
        delay_secs: u64,
    },
    HookTriggered {
        name: String,
    },
    Stopped {
        reason: String,
    },
    SpawnError {
        error: String,
    },
    TaskError {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitStatus {
    Success,
    Failure,
}

// === Directory/Path Helpers ===

fn tend_dir() -> io::Result<PathBuf> {
    let home = dirs_next::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find home directory"))?;
    Ok(home.join(".tend"))
}

/// Returns the log directory for a specific job, creating it if needed.
pub fn job_log_dir(job_name: &str) -> io::Result<PathBuf> {
    let dir = tend_dir()?.join("logs").join(job_name);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

// === Log File Management ===

/// A handle for writing output lines to a log file on disk.
#[derive(Debug)]
pub struct RunLogger {
    output_writer: BufWriter<File>,
    meta_path: PathBuf,
    #[allow(dead_code)]
    log_path: PathBuf,
    run_id: u64,
}

impl RunLogger {
    /// Create a new `RunLogger` for a job run.
    /// Creates a timestamped `.log` file and ensures `meta.jsonl` exists.
    pub fn new(job_name: &str, run_id: u64) -> io::Result<Self> {
        let dir = job_log_dir(job_name)?;
        let timestamp = format_timestamp(SystemTime::now());
        let log_path = dir.join(format!("{timestamp}.log"));
        let meta_path = dir.join("meta.jsonl");

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let output_writer = BufWriter::new(log_file);

        Ok(Self {
            output_writer,
            meta_path,
            log_path,
            run_id,
        })
    }

    /// Returns the path to the current run's log file.
    #[allow(dead_code)]
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Write an output line (stdout or stderr) to the log file.
    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        writeln!(self.output_writer, "{line}")?;
        // Flush periodically isn't needed - BufWriter flushes at 8KB by default,
        // and we flush on drop. For crash safety we can flush after system events.
        Ok(())
    }

    /// Flush buffered output to disk.
    pub fn flush(&mut self) -> io::Result<()> {
        self.output_writer.flush()
    }

    /// Write a metadata event to `meta.jsonl`.
    pub fn write_meta(&self, kind: MetaEventKind) -> io::Result<()> {
        let entry = MetaEntry {
            run_id: self.run_id,
            at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            kind,
        };

        let mut meta_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.meta_path)?;

        let json = serde_json::to_string(&entry)
            .map_err(io::Error::other)?;
        writeln!(meta_file, "{json}")?;
        Ok(())
    }
}

impl Drop for RunLogger {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

// === Scrollback: Reading from disk ===

/// Read the last `n` lines from a log file.
/// Returns lines in order (oldest to newest).
#[allow(dead_code)]
pub fn read_tail(log_path: &Path, n: usize) -> io::Result<Vec<String>> {
    let file = File::open(log_path)?;
    let reader = io::BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().collect::<io::Result<Vec<_>>>()?;
    let start = all_lines.len().saturating_sub(n);
    Ok(all_lines.get(start..).unwrap_or_default().to_vec())
}

/// Read a range of lines from a log file.
/// `start` is 0-indexed from the beginning of the file.
/// Returns up to `count` lines starting from `start`.
#[allow(dead_code)]
pub fn read_lines_range(log_path: &Path, start: usize, count: usize) -> io::Result<Vec<String>> {
    let file = File::open(log_path)?;
    let reader = io::BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .skip(start)
        .take(count)
        .collect::<io::Result<Vec<_>>>()?;
    Ok(lines)
}

/// Count total lines in a log file.
#[allow(dead_code)]
pub fn count_lines(log_path: &Path) -> io::Result<usize> {
    let file = File::open(log_path)?;
    let reader = io::BufReader::new(file);
    Ok(reader.lines().count())
}

// === Retention Cleanup ===

/// Delete log files older than `retention_days` and prune old entries from `meta.jsonl`.
pub fn cleanup_retention(retention_days: u64) {
    let Ok(tend) = tend_dir() else {
        return;
    };

    let logs_dir = tend.join("logs");
    if !logs_dir.exists() {
        return;
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retention_days * 24 * 3600))
        .unwrap_or(UNIX_EPOCH);

    let cutoff_secs = cutoff
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let Ok(entries) = fs::read_dir(&logs_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Clean up old .log files in this job's directory
        cleanup_job_log_dir(&path, cutoff);

        // Prune old entries from meta.jsonl
        prune_meta_jsonl(&path.join("meta.jsonl"), cutoff_secs);
    }
}

fn cleanup_job_log_dir(dir: &Path, cutoff: SystemTime) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }

        let Ok(metadata) = path.metadata() else {
            continue;
        };

        let Ok(modified) = metadata.modified() else {
            continue;
        };

        if modified < cutoff {
            let _ = fs::remove_file(&path);
        }
    }

    // Remove the directory if empty (ignore errors)
    let _ = fs::remove_dir(dir);
}

fn prune_meta_jsonl(meta_path: &Path, cutoff_secs: u64) {
    let Ok(file) = File::open(meta_path) else {
        return;
    };

    let reader = io::BufReader::new(file);
    let mut retained = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        if let Ok(entry) = serde_json::from_str::<MetaEntry>(&line)
            && entry.at >= cutoff_secs
        {
            retained.push(line);
        }
    }

    if let Ok(mut file) = File::create(meta_path) {
        for line in &retained {
            let _ = writeln!(file, "{line}");
        }
    }
}
