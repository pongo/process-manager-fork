use crate::types::ProcessMetrics;
use std::collections::{HashMap, HashSet};
use sysinfo::{Pid, ProcessRefreshKind, System};

pub struct Monitoring;

impl Monitoring {
    /// Call with a persistent `System` that was created at startup.
    /// Each call refreshes it so the CPU delta is computed from the
    /// previous refresh (accurate after the first measurement).
    pub fn get_process_metrics(pid: u32, system: &mut System) -> Result<ProcessMetrics, String> {
        system.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu().with_memory());
        tree_metrics(
            Pid::from_u32(pid),
            system.total_memory(),
            system.processes().iter().map(|(&pid, process)| {
                (pid, process.parent(), process.cpu_usage(), process.memory())
            }),
        )
    }

    #[allow(dead_code)]
    pub fn get_all_processes_metrics(
        system: &mut System,
    ) -> Result<Vec<(u32, ProcessMetrics)>, String> {
        system.refresh_all();
        let total_memory = system.total_memory();
        let mut metrics = Vec::new();

        for (pid, process) in system.processes() {
            let cpu_percent = process.cpu_usage();
            let memory_bytes = process.memory();
            let memory_mb = (memory_bytes as f64 / (1024.0 * 1024.0)).ceil() as u64;
            let memory_percent = if total_memory > 0 {
                (memory_bytes as f64 / total_memory as f64 * 100.0) as f32
            } else {
                0.0
            };

            metrics.push((
                pid.as_u32(),
                ProcessMetrics {
                    cpu_percent,
                    memory_mb,
                    memory_percent,
                },
            ));
        }

        Ok(metrics)
    }
}

fn tree_metrics(
    root: Pid,
    total_memory: u64,
    processes: impl IntoIterator<Item = (Pid, Option<Pid>, f32, u64)>,
) -> Result<ProcessMetrics, String> {
    let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
    let mut resources = HashMap::new();
    for (pid, parent, cpu, memory) in processes {
        resources.insert(pid, (cpu, memory));
        if let Some(parent) = parent {
            children.entry(parent).or_default().push(pid);
        }
    }
    if !resources.contains_key(&root) {
        return Err("Process not found".to_string());
    }

    let mut pending = vec![root];
    let mut visited = HashSet::new();
    let mut cpu_percent = 0.0;
    let mut memory_bytes = 0_u64;
    while let Some(pid) = pending.pop() {
        if !visited.insert(pid) {
            continue;
        }
        if let Some(&(cpu, memory)) = resources.get(&pid) {
            cpu_percent += cpu;
            memory_bytes += memory;
        }
        if let Some(descendants) = children.get(&pid) {
            pending.extend(descendants.iter().copied());
        }
    }

    Ok(ProcessMetrics {
        cpu_percent,
        memory_mb: (memory_bytes as f64 / (1024.0 * 1024.0)).ceil() as u64,
        memory_percent: if total_memory > 0 {
            (memory_bytes as f64 / total_memory as f64 * 100.0) as f32
        } else {
            0.0
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_live_process_memory_and_cpu() {
        let mut system = System::new();
        system.refresh_memory();
        let pid = std::process::id();
        let first = Monitoring::get_process_metrics(pid, &mut system).unwrap();
        assert!(first.memory_mb > 0, "Live process memory must be nonzero");
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(300) {
            std::hint::black_box((0..1000).sum::<u64>());
        }
        let second = Monitoring::get_process_metrics(pid, &mut system).unwrap();
        assert!(second.memory_mb > 0);
        assert!(second.cpu_percent > 0.0, "Busy process CPU must be nonzero");
    }

    fn snapshot(
        pid: u32,
        parent: Option<u32>,
        cpu: f32,
        memory: u64,
    ) -> (Pid, Option<Pid>, f32, u64) {
        (Pid::from_u32(pid), parent.map(Pid::from_u32), cpu, memory)
    }

    #[test]
    fn sums_only_root_and_all_descendants() {
        let mb = 1024 * 1024;
        let metrics = tree_metrics(
            Pid::from_u32(1),
            100 * mb,
            [
                snapshot(1, None, 1.0, 9 * mb),
                snapshot(2, Some(1), 20.0, 40 * mb),
                snapshot(3, Some(1), 30.0, mb),
                snapshot(4, Some(2), 60.0, 2 * mb),
                snapshot(5, None, 90.0, 80 * mb),
                snapshot(6, Some(5), 90.0, 80 * mb),
            ],
        )
        .unwrap();
        assert_eq!(metrics.memory_mb, 52);
        assert_eq!(metrics.cpu_percent, 111.0);
        assert_eq!(metrics.memory_percent, 52.0);
    }

    #[test]
    fn rounds_after_summing_and_handles_cycles() {
        let metrics = tree_metrics(
            Pid::from_u32(1),
            0,
            [snapshot(1, Some(2), 1.0, 1), snapshot(2, Some(1), 2.0, 1)],
        )
        .unwrap();
        assert_eq!(metrics.memory_mb, 1);
        assert_eq!(metrics.cpu_percent, 3.0);
        assert_eq!(metrics.memory_percent, 0.0);
    }

    #[test]
    fn reflects_children_added_and_removed_between_snapshots() {
        let root = snapshot(1, None, 1.0, 1024 * 1024);
        let child = snapshot(2, Some(1), 2.0, 1024 * 1024);
        for (processes, expected_mb) in [(vec![root], 1), (vec![root, child], 2), (vec![root], 1)] {
            let metrics = tree_metrics(Pid::from_u32(1), 0, processes).unwrap();
            assert_eq!(metrics.memory_mb, expected_mb);
            assert_eq!(
                metrics.cpu_percent,
                if expected_mb == 1 { 1.0 } else { 3.0 }
            );
        }
    }

    #[test]
    fn rejects_missing_root_even_when_children_exist() {
        assert!(tree_metrics(Pid::from_u32(1), 0, [snapshot(2, Some(1), 1.0, 1)]).is_err());
    }
}
