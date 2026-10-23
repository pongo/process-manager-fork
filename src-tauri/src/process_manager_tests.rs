use super::*;
use std::io::{BufRead, BufReader};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

// Three generations, with redirected leaf output so the only stdout lines are PIDs.
const TREE_SCRIPT: &str = r#"
$script = @'
$i = [Diagnostics.ProcessStartInfo]::new('ping.exe', '-n 60 127.0.0.1');
$i.UseShellExecute = $false; $i.CreateNoWindow = $true;
$i.RedirectStandardOutput = $true; $i.RedirectStandardError = $true;
$p = [Diagnostics.Process]::Start($i); $p.Id; Start-Sleep -Seconds 60
'@
$i = [Diagnostics.ProcessStartInfo]::new('powershell.exe');
$i.Arguments = '-NoProfile -NonInteractive -Command ' + $script;
$i.UseShellExecute = $false; $i.CreateNoWindow = $true;
$i.RedirectStandardOutput = $true; $i.RedirectStandardError = $true;
$p = [Diagnostics.Process]::Start($i); $p.Id; $p.StandardOutput.ReadLine();
"#;

fn add_tree(manager: &mut ProcessManager, exit_root: bool) -> String {
    let script = if exit_root {
        TREE_SCRIPT.to_string()
    } else {
        format!("{TREE_SCRIPT}\nStart-Sleep -Seconds 60")
    };
    manager.add_process(
        "tree-test".into(),
        "powershell.exe".into(),
        vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script,
        ],
        None,
        false,
    )
}

fn track_tree(pid: u32, stdout: ChildStdout) -> Vec<u32> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let pids: Vec<u32> = BufReader::new(stdout)
            .lines()
            .take(2)
            .map(|line| line.unwrap().trim().parse().unwrap())
            .collect();
        let _ = tx.send(pids);
    });
    let mut pids = rx
        .recv_timeout(Duration::from_secs(15))
        .expect("Tree did not start");
    assert_eq!(pids.len(), 2);
    pids.push(pid);
    pids
}

fn start_tree(manager: &mut ProcessManager, id: &str) -> Vec<u32> {
    let (pid, stdout, _) = manager.spawn_process(id).unwrap();
    track_tree(pid, stdout)
}

fn assert_stopped(pids: &[u32]) {
    let mut system = sysinfo::System::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    for &pid in pids {
        while system.refresh_process(sysinfo::Pid::from_u32(pid)) {
            assert!(Instant::now() < deadline, "Process {pid} is still running");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
fn stop_and_remove_terminate_complete_trees() {
    let mut manager = ProcessManager::new();
    let id = add_tree(&mut manager, false);
    let handles = start_tree(&mut manager, &id);
    manager.stop_process(&id).unwrap();
    assert_stopped(&handles);
    assert!(matches!(
        manager.processes[&id].status,
        ProcessStatus::Stopped
    ));
    assert!(manager.processes[&id].pid.is_none());
    manager.stop_process(&id).unwrap();
    let handles = start_tree(&mut manager, &id);
    manager.remove_process(&id).unwrap();
    assert_stopped(&handles);
    assert!(!manager.processes.contains_key(&id));
}

#[test]
fn stop_terminates_descendants_after_root_exit() {
    let mut manager = ProcessManager::new();
    let id = add_tree(&mut manager, true);
    let handles = start_tree(&mut manager, &id);
    manager
        .processes
        .get_mut(&id)
        .unwrap()
        .child
        .as_mut()
        .unwrap()
        .inner_mut()
        .wait()
        .unwrap();
    manager.stop_process(&id).unwrap();
    assert_stopped(&handles);
}

#[test]
fn restart_terminates_previous_tree() {
    let mut manager = ProcessManager::new();
    let id = add_tree(&mut manager, false);
    let old = start_tree(&mut manager, &id);
    let (pid, stdout, _) = manager.restart_process(&id).unwrap();
    assert_stopped(&old);
    let new = track_tree(pid, stdout);
    manager.stop_process(&id).unwrap();
    assert_stopped(&new);
}

#[test]
fn stop_all_allows_subsequent_starts_but_shutdown_blocks_them() {
    let mut manager = ProcessManager::new();
    let a = add_tree(&mut manager, false);
    let b = add_tree(&mut manager, true);
    let a_handles = start_tree(&mut manager, &a);
    let b_handles = start_tree(&mut manager, &b);
    manager.stop_all().unwrap();
    assert_stopped(&a_handles);
    assert_stopped(&b_handles);
    let handles = start_tree(&mut manager, &a);
    let manager = Arc::new(Mutex::new(manager));
    manager.lock().unwrap().shutdown().unwrap();
    assert_stopped(&handles);
    let other = Arc::clone(&manager);
    std::thread::spawn(move || {
        let mut manager = other.lock().unwrap();
        assert!(manager.spawn_process(&a).is_err());
        assert!(manager.restart_process(&b).is_err());
        manager.shutdown().unwrap();
    })
    .join()
    .unwrap();
}

#[test]
fn stop_failures_preserve_processes_and_do_not_skip_other_trees() {
    let mut manager = ProcessManager::new();
    let a = add_tree(&mut manager, false);
    let b = add_tree(&mut manager, false);
    let a_handles = start_tree(&mut manager, &a);
    let b_handles = start_tree(&mut manager, &b);
    let original_child = manager.processes.get_mut(&a).unwrap().child.take().unwrap();
    manager.processes.get_mut(&a).unwrap().child = Some(Box::new(FailingStop(original_child)));
    assert!(manager.remove_process(&a).is_err());
    assert!(manager.restart_process(&a).is_err());
    assert!(manager.shutdown().is_err());
    assert!(matches!(
        manager.processes[&a].status,
        ProcessStatus::Running
    ));
    assert!(manager.processes[&a].child.is_some());
    assert!(manager.processes[&a].pid.is_some());
    assert!(!manager.shutting_down);
    assert_stopped(&b_handles);
    let failing_child = manager.processes.get_mut(&a).unwrap().child.take().unwrap();
    manager.processes.get_mut(&a).unwrap().child = Some(failing_child.into_inner());
    manager.shutdown().unwrap();
    assert_stopped(&a_handles);
}

#[derive(Debug)]
struct FailingStop(Box<dyn ChildWrapper>);

impl ChildWrapper for FailingStop {
    fn inner(&self) -> &dyn ChildWrapper {
        self.0.as_ref()
    }

    fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
        self.0.as_mut()
    }

    fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
        self.0
    }

    fn start_kill(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Stop denied",
        ))
    }
}
