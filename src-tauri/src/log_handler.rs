use crate::types::LogEntry;
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct LogHandler {
    log_dir: PathBuf,
}

impl LogHandler {
    pub fn new(log_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        fs::create_dir_all(&log_dir)?;
        Ok(LogHandler { log_dir })
    }

    pub fn get_log_file(&self, process_id: &str) -> PathBuf {
        self.log_dir.join(format!("{}.log", process_id))
    }

    pub fn write_log(&self, process_id: &str, level: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let log_path = self.get_log_file(process_id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] [{}] {}\n", timestamp, level, message);
        file.write_all(log_line.as_bytes())?;

        Ok(())
    }

    pub fn read_logs(&self, process_id: &str, limit: Option<usize>) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
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
        let log_path = self.get_log_file(process_id);
        if log_path.exists() {
            fs::remove_file(log_path)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn rotate_log(&self, process_id: &str, max_size_mb: u64) -> Result<(), Box<dyn std::error::Error>> {
        let log_path = self.get_log_file(process_id);
        
        if log_path.exists() {
            let metadata = std::fs::metadata(&log_path)?;
            let size_mb = metadata.len() / (1024 * 1024);
            
            if size_mb > max_size_mb {
                let timestamp = Local::now().format("%Y%m%d_%H%M%S");
                let backup_name = format!("{}.{}.log", process_id, timestamp);
                let backup_path = self.log_dir.join(backup_name);
                fs::rename(&log_path, &backup_path)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LogHandler;

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
