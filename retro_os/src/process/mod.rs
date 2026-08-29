//! RetroOS Process Manager
//! 
//! Handles Linux program execution, process lifecycle, and resource management.
//! Priority #1: Native Linux program execution

use anyhow::{Result, Context};
use log::{info, debug, error, warn};
use nix::unistd::{Pid, fork, ForkResult, execvp, close};
use nix::sys::wait::{waitpid, WaitStatus};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Process status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Exited(i32),
    Failed(String),
}

/// Process information
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub args: Vec<String>,
    pub status: ProcessStatus,
    pub start_time: u64,
    pub memory_usage_kb: usize,
    pub cpu_usage_percent: f32,
}

/// Process manager for handling Linux programs
pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<i32, ProcessInfo>>>,
    process_count: Arc<RwLock<usize>>,
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        ProcessManager {
            processes: Arc::new(RwLock::new(HashMap::new())),
            process_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Execute a Linux program
    pub async fn execute(&self, program: &str, args: &[&str]) -> Result<i32> {
        info!("Executing program: {} with args: {:?}", program, args);

        let c_program = CString::new(program)
            .with_context(|| format!("Invalid program name: {}", program))?;
        let c_args: Vec<CString> = args
            .iter()
            .map(|&s| CString::new(s))
            .collect::<Result<Vec<_>, _>>()?;
        let c_args_refs: Vec<&CString> = c_args.iter().collect();

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // Child process
                let mut exec_args = vec![&c_program];
                exec_args.extend(c_args_refs);
                
                if let Err(e) = execvp(&c_program, &exec_args) {
                    eprintln!("Failed to execute {}: {}", program, e);
                    std::process::exit(127);
                }
                unreachable!();
            }
            Ok(ForkResult::Parent { child }) => {
                // Parent process
                info!("Spawned child process with PID: {}", child);
                
                let process_info = ProcessInfo {
                    pid: child.as_raw(),
                    name: program.to_string(),
                    args: args.iter().map(|s| s.to_string()).collect(),
                    status: ProcessStatus::Running,
                    start_time: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                    memory_usage_kb: 0,
                    cpu_usage_percent: 0.0,
                };

                {
                    let mut processes = self.processes.write().await;
                    processes.insert(child.as_raw(), process_info);
                    *self.process_count.write().await += 1;
                }

                Ok(child.as_raw())
            }
            Err(err) => {
                error!("Fork failed: {}", err);
                Err(anyhow::anyhow!("Fork failed: {}", err))
            }
        }
    }

    /// Wait for a process to complete
    pub async fn wait_for_process(&self, pid: i32) -> Result<i32> {
        info!("Waiting for process {} to complete", pid);
        
        let pid_obj = Pid::from_raw(pid);
        match waitpid(pid_obj, None) {
            Ok(WaitStatus::Exited(_, exit_code)) => {
                info!("Process {} exited with code {}", pid, exit_code);
                
                {
                    let mut processes = self.processes.write().await;
                    if let Some(proc) = processes.get_mut(&pid) {
                        proc.status = ProcessStatus::Exited(exit_code);
                    }
                    *self.process_count.write().await -= 1;
                }
                
                Ok(exit_code)
            }
            Ok(WaitStatus::Signaled(_, signal, _)) => {
                warn!("Process {} killed by signal {}", pid, signal);
                
                {
                    let mut processes = self.processes.write().await;
                    if let Some(proc) = processes.get_mut(&pid) {
                        proc.status = ProcessStatus::Failed(format!("Killed by signal {}", signal));
                    }
                    *self.process_count.write().await -= 1;
                }
                
                Ok(-1)
            }
            Ok(status) => {
                debug!("Process {} status: {:?}", pid, status);
                Ok(0)
            }
            Err(e) => {
                error!("Waitpid failed for {}: {}", pid, e);
                Err(anyhow::anyhow!("Waitpid failed: {}", e))
            }
        }
    }

    /// Get process information
    pub async fn get_process(&self, pid: i32) -> Option<ProcessInfo> {
        let processes = self.processes.read().await;
        processes.get(&pid).cloned()
    }

    /// List all running processes
    pub async fn list_processes(&self) -> Vec<ProcessInfo> {
        let processes = self.processes.read().await;
        processes.values().cloned().collect()
    }

    /// Get total process count
    pub async fn process_count(&self) -> usize {
        *self.process_count.read().await
    }

    /// Update process memory usage
    pub async fn update_memory_usage(&self, pid: i32) -> Result<usize> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let status_path = format!("/proc/{}/status", pid);
            if let Ok(status) = fs::read_to_string(&status_path) {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let rss_kb: usize = parts[1].parse().unwrap_or(0);
                            if let Some(proc) = self.get_process(pid).await {
                                let mut processes = self.processes.write().await;
                                if let Some(p) = processes.get_mut(&pid) {
                                    p.memory_usage_kb = rss_kb;
                                }
                            }
                            return Ok(rss_kb);
                        }
                    }
                }
            }
        }
        Ok(0)
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_manager_creation() {
        let pm = ProcessManager::new();
        assert_eq!(pm.process_count().await, 0);
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        let pm = ProcessManager::new();
        
        // Test with 'true' command which always succeeds
        let result = pm.execute("/bin/true", &[]).await;
        assert!(result.is_ok());
        
        let pid = result.unwrap();
        let exit_code = pm.wait_for_process(pid).await;
        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }
}
