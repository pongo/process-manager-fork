use crate::types::*;
use process_wrap::std::{ChildWrapper, CommandWrap};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[cfg(target_os = "windows")]
use process_wrap::std::{CreationFlags, JobObject};

pub struct ProcessInstance {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub status: ProcessStatus,
    pub child: Option<Box<dyn ChildWrapper>>,
    pub pid: Option<u32>,
    pub auto_restart: bool,
    pub auto_start: bool,
    pub working_dir: Option<String>,
    pub start_time: Option<u64>,
    pub crash_count: u32,
    pub should_restart: Arc<AtomicBool>,
}

impl ProcessInstance {
    pub fn new(name: String, command: String, args: Vec<String>, auto_restart: bool, auto_start: bool) -> Self {
        ProcessInstance {
            id: Uuid::new_v4().to_string(),
            name,
            command,
            args,
            status: ProcessStatus::Stopped,
            child: None,
            pid: None,
            auto_restart,
            auto_start,
            working_dir: None,
            start_time: None,
            crash_count: 0,
            should_restart: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn from_config(config: &ProcessConfig) -> Self {
        ProcessInstance {
            id: config.id.clone(),
            name: config.name.clone(),
            command: config.command.clone(),
            args: config.args.clone(),
            status: ProcessStatus::Stopped,
            child: None,
            pid: None,
            auto_restart: config.auto_restart,
            auto_start: config.auto_start,
            working_dir: config.working_dir.clone(),
            start_time: None,
            crash_count: 0,
            should_restart: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_uptime_ms(&self) -> u64 {
        match self.start_time {
            Some(start) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                now.saturating_sub(start)
            }
            None => 0,
        }
    }

    pub fn to_state(&self) -> ProcessState {
        ProcessState {
            id: self.id.clone(),
            name: self.name.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            status: self.status.to_string(),
            pid: self.pid,
            auto_restart: self.auto_restart,
            auto_start: self.auto_start,
            working_dir: self.working_dir.clone(),
            uptime_ms: self.get_uptime_ms(),
            crash_count: self.crash_count,
        }
    }

    pub fn to_config(&self) -> ProcessConfig {
        ProcessConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            auto_restart: self.auto_restart,
            auto_start: self.auto_start,
            working_dir: self.working_dir.clone(),
            env: None,
        }
    }
}

pub struct ProcessManager {
    pub processes: HashMap<String, ProcessInstance>,
    shutting_down: bool,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            processes: HashMap::new(),
            shutting_down: false,
        }
    }

    pub fn add_process(&mut self, name: String, command: String, args: Vec<String>, working_dir: Option<String>, auto_restart: bool) -> String {
        let mut process = ProcessInstance::new(name, command, args, auto_restart, false);
        process.working_dir = working_dir;
        let id = process.id.clone();
        self.processes.insert(id.clone(), process);
        id
    }

    pub fn get_process(&self, id: &str) -> Option<&ProcessInstance> {
        self.processes.get(id)
    }

    pub fn get_process_mut(&mut self, id: &str) -> Option<&mut ProcessInstance> {
        self.processes.get_mut(id)
    }

    pub fn remove_process(&mut self, id: &str) -> Result<(), String> {
        self.stop_process(id)?;
        self.processes.remove(id);
        Ok(())
    }

    pub fn get_all_processes(&self) -> Vec<ProcessState> {
        self.processes.values().map(|p| p.to_state()).collect()
    }

    pub fn spawn_process(&mut self, id: &str) -> Result<(u32, ChildStdout, ChildStderr), String> {
        if self.shutting_down {
            return Err("Application is shutting down".to_string());
        }
        let process = self
            .get_process_mut(id)
            .ok_or_else(|| "Process not found".to_string())?;

        // Don't spawn if already running
        if matches!(process.status, ProcessStatus::Running) {
            return Err("Process is already running".to_string());
        }

        let mut executable = PathBuf::from(&process.command);
        if executable.is_relative() {
            if let Some(ref dir) = process.working_dir {
                let working_dir = PathBuf::from(dir);
                let working_dir = if working_dir.is_absolute() {
                    working_dir
                } else {
                    std::env::current_dir()
                        .map_err(|e| format!("Failed to resolve working directory: {}", e))?
                        .join(working_dir)
                };
                let candidate = working_dir.join(&executable);
                if candidate.is_file() {
                    executable = candidate;
                }
            }
        }

        let mut cmd = Command::new(&executable);
        cmd.args(&process.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        if let Some(ref dir) = process.working_dir {
            cmd.current_dir(dir);
        }

        let mut command = CommandWrap::from(cmd);
        #[cfg(target_os = "windows")]
        command
            .wrap(CreationFlags(
                windows::Win32::System::Threading::CREATE_NO_WINDOW,
            ))
            .wrap(JobObject);

        match command.spawn() {
            Ok(mut child) => {
                let pid = child.id();
                let stdout = child.stdout().take()
                    .ok_or_else(|| "Failed to capture stdout".to_string())?;
                let stderr = child.stderr().take()
                    .ok_or_else(|| "Failed to capture stderr".to_string())?;
                process.pid = Some(pid);
                process.status = ProcessStatus::Running;
                process.child = Some(child);
                process.start_time = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                );
                process.crash_count = 0;
                process.should_restart.store(true, Ordering::SeqCst);
                Ok((pid, stdout, stderr))
            }
            Err(e) => {
                process.status = ProcessStatus::Crashed;
                process.crash_count += 1;
                Err(format!("Failed to spawn process: {}", e))
            }
        }
    }

    pub fn stop_process(&mut self, id: &str) -> Result<(), String> {
        let process = self
            .get_process_mut(id)
            .ok_or_else(|| "Process not found".to_string())?;

        process.should_restart.store(false, Ordering::SeqCst);

        if let Some(child) = process.child.as_mut() {
            child.start_kill().map_err(|e| format!("Failed to stop process tree: {e}"))?;
            // Stop through the wrapper; wait only for its root process.
            child.inner_mut().wait().map_err(|e| format!("Failed to wait for process: {e}"))?;
        }

        process.child = None;
        process.status = ProcessStatus::Stopped;
        process.pid = None;
        process.start_time = None;
        Ok(())
    }

    pub fn restart_process(&mut self, id: &str) -> Result<(u32, ChildStdout, ChildStderr), String> {
        if self.shutting_down {
            return Err("Application is shutting down".to_string());
        }
        self.stop_process(id)?;
        self.spawn_process(id)
    }

    pub fn stop_all(&mut self) -> Result<(), String> {
        let ids: Vec<_> = self.processes.keys().cloned().collect();
        let mut errors = Vec::new();
        for id in ids {
            if let Err(error) = self.stop_process(&id) {
                errors.push(format!("{}: {error}", self.processes[&id].name));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        self.shutting_down = true;
        let result = self.stop_all();
        if result.is_err() {
            self.shutting_down = false;
        }
        result
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, target_os = "windows"))]
#[path = "process_manager_tests.rs"]
mod tests;
