use crate::types::LogEntry;
use chrono::Local;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const MAX_LOG_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_LOG_BACKUPS: usize = 1;

pub struct LogHandler {
    log_dir: PathBuf,
    process_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl LogHandler {
    pub fn new(log_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        fs::create_dir_all(&log_dir)?;
        Ok(LogHandler {
            log_dir,
            process_locks: Mutex::new(HashMap::new()),
        })
    }

    pub fn get_log_file(&self, process_id: &str) -> PathBuf {
        self.log_dir.join(format!("{}.log", process_id))
    }

    pub fn write_log(&self, process_id: &str, level: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let process_lock = self.process_lock(process_id);
        let _guard = process_lock.lock().unwrap_or_else(|e| e.into_inner());

        let log_path = self.get_log_file(process_id);
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] [{}] {}\n", timestamp, level, message);

        if log_path.exists() {
            let current_size = fs::metadata(&log_path)?.len();
            // Preserve oversized messages intact without archiving an empty file.
            if current_size > 0 && current_size.saturating_add(log_line.len() as u64) > MAX_LOG_SIZE_BYTES {
                self.rotate_log(process_id)?;
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        file.write_all(log_line.as_bytes())?;

        Ok(())
    }

    pub fn read_logs(&self, process_id: &str, limit: Option<usize>) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
        let process_lock = self.process_lock(process_id);
        let _guard = process_lock.lock().unwrap_or_else(|e| e.into_inner());
        let log_path = self.get_log_file(process_id);
        
        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&log_path)?;
        let mut logs: Vec<LogEntry> = content
            .lines()
            .filter_map(|line| {
                // Parse format: [timestamp] [level] message
                let (timestamp, rest) = line.strip_prefix('[')?.split_once("] [")?;
                let (level, message) = rest.split_once("] ")?;
                Some(LogEntry {
                    timestamp: timestamp.to_string(),
                    level: level.to_string(),
                    message: message.to_string(),
                })
            })
            .collect();

        // Apply limit (last N entries)
        if let Some(limit_val) = limit {
            if logs.len() > limit_val {
                let start = logs.len() - limit_val;
                logs = logs[start..].to_vec();
            }
        }

        Ok(logs)
    }

    pub fn clear_logs(&self, process_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let process_lock = self.process_lock(process_id);
        let _guard = process_lock.lock().unwrap_or_else(|e| e.into_inner());
        let log_path = self.get_log_file(process_id);
        if log_path.exists() {
            fs::remove_file(log_path)?;
        }
        Ok(())
    }

    fn process_lock(&self, process_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.process_locks.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(locks.entry(process_id.to_string()).or_insert_with(|| Arc::new(Mutex::new(()))))
    }

    fn rotate_log(&self, process_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let log_path = self.get_log_file(process_id);

        if !log_path.exists() {
            return Ok(());
        }

        for index in (1..=MAX_LOG_BACKUPS).rev() {
            let backup_path = self.log_dir.join(format!("{}.{}.log", process_id, index));
            if index == MAX_LOG_BACKUPS {
                if backup_path.exists() {
                    fs::remove_file(&backup_path)?;
                }
            } else {
                let older_backup = self.log_dir.join(format!("{}.{}.log", process_id, index + 1));
                if backup_path.exists() {
                    fs::rename(&backup_path, &older_backup)?;
                }
            }
        }

        let backup_path = self.log_dir.join(format!("{}.1.log", process_id));
        fs::rename(log_path, backup_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{LogHandler, MAX_LOG_SIZE_BYTES};

    #[test]
    fn rotation_preserves_exact_limit_and_replaces_the_only_backup() {
        let log_dir = std::env::temp_dir().join(format!("pm-logs-{}", uuid::Uuid::new_v4()));
        let handler = LogHandler::new(log_dir.clone()).unwrap();
        // Timestamp, level, delimiters and newline occupy 36 bytes for stdout.
        let message = "a".repeat(MAX_LOG_SIZE_BYTES as usize - 36);
        handler.write_log("process", "stdout", &message).unwrap();
        let active_path = handler.get_log_file("process");
        let backup_path = log_dir.join("process.1.log");
        assert_eq!(std::fs::metadata(&active_path).unwrap().len(), MAX_LOG_SIZE_BYTES);
        assert!(!backup_path.exists());

        handler.write_log("process", "stderr", "next").unwrap();
        assert_eq!(std::fs::metadata(&backup_path).unwrap().len(), MAX_LOG_SIZE_BYTES);
        assert_eq!(handler.read_logs("process", None).unwrap()[0].message, "next");

        handler.write_log("process", "stdout", &message).unwrap();
        let backup = std::fs::read_to_string(&backup_path).unwrap();
        assert!(backup.ends_with("[stderr] next\n"));
        assert_eq!(std::fs::read_dir(&log_dir).unwrap().count(), 2);
        std::fs::remove_dir_all(log_dir).unwrap();
    }

    #[test]
    fn reading_saved_logs_preserves_levels_and_messages() {
        let log_dir = std::env::temp_dir().join(format!("pm-logs-{}", uuid::Uuid::new_v4()));
        let handler = LogHandler::new(log_dir.clone()).unwrap();
        let messages = [
            ("stdout", "Server listening on http://127.0.0.1:53680/"),
            ("stderr", "Ignoring unsupported file: inputs 2026-09-30.7z"),
            ("stdout", "  [nested] message with trailing spaces  "),
            ("stdout", ""),
        ];

        for (level, message) in messages {
            handler.write_log("process", level, message).unwrap();
        }

        let logs = handler.read_logs("process", None).unwrap();
        std::fs::remove_dir_all(log_dir).unwrap();

        assert_eq!(logs.len(), messages.len());
        for (log, (level, message)) in logs.iter().zip(messages) {
            assert_eq!(log.level, level);
            assert_eq!(log.message, message);
            assert!(!log.timestamp.is_empty());
        }
    }
}
