use crate::runtime::l400_run_dir;
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CgroupError {
    #[error("Cgroups v2 not available on this system")]
    NotAvailable,
    #[error("Failed to create slice: {0}")]
    SliceCreation(String),
    #[error("Failed to assign process: {0}")]
    AssignmentFailed(String),
    #[error("Permission denied (requires root or l400 group)")]
    PermissionDenied,
    #[error("Invalid job registry entry: {0}")]
    InvalidJob(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkloadType {
    Interactive,
    Batch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
    JobQ,
    Active,
    Completed,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::JobQ => write!(f, "JOBQ"),
            JobStatus::Active => write!(f, "ACTIVE"),
            JobStatus::Completed => write!(f, "COMPLETED"),
            JobStatus::Failed => write!(f, "FAILED"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = CgroupError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "JOBQ" => Ok(JobStatus::JobQ),
            "ACTIVE" => Ok(JobStatus::Active),
            "COMPLETED" => Ok(JobStatus::Completed),
            "FAILED" => Ok(JobStatus::Failed),
            _ => Err(CgroupError::InvalidJob(format!("invalid status: {}", s))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CgroupParams {
    pub cpu_weight: u64,
    pub cpu_max: String,
    pub io_weight: u64,
    pub memory_high: String,
    pub memory_max: String,
    pub pids_max: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkloadJob {
    pub pid: u64,
    pub name: String,
    pub user: String,
    pub workload: WorkloadType,
    pub status: JobStatus,
    pub subsystem: String,
    pub command: String,
}

impl Default for CgroupParams {
    fn default() -> Self {
        Self {
            cpu_weight: 100,
            cpu_max: String::from("100000 100000"),
            io_weight: 100,
            memory_high: String::from("524288000"),
            memory_max: String::from("1073741824"),
            pids_max: 1024,
        }
    }
}

impl CgroupParams {
    pub fn interactive() -> Self {
        Self {
            cpu_weight: 10000,
            cpu_max: String::from("100000 100000"),
            io_weight: 100,
            memory_high: String::from("536870912"),
            memory_max: String::from("1073741824"),
            pids_max: 512,
        }
    }

    pub fn batch() -> Self {
        Self {
            cpu_weight: 100,
            cpu_max: String::from("10000 100000"),
            io_weight: 50,
            memory_high: String::from("1073741824"),
            memory_max: String::from("4294967296"),
            pids_max: 2048,
        }
    }
}

const L400_CGROUP_ROOT: &str = "/sys/fs/cgroup/l400.slice";
const QINTER_SLICE: &str = "l400.qinter";
const QBATCH_SLICE: &str = "l400.qbatch";
fn l400_root() -> PathBuf {
    PathBuf::from(L400_CGROUP_ROOT)
}

fn job_registry_path(base: &Path) -> PathBuf {
    base.join("jobs")
}

fn workload_name(workload: WorkloadType) -> &'static str {
    match workload {
        WorkloadType::Interactive => "QINTER",
        WorkloadType::Batch => "QBATCH",
    }
}

fn workload_from_name(value: &str) -> Result<WorkloadType, CgroupError> {
    match value {
        "QINTER" | "INTERACTIVE" => Ok(WorkloadType::Interactive),
        "QBATCH" | "BATCH" => Ok(WorkloadType::Batch),
        other => Err(CgroupError::InvalidJob(other.to_string())),
    }
}

fn job_file(base: &Path, pid: u64) -> PathBuf {
    job_registry_path(base).join(format!("{pid}.job"))
}

fn current_user_name() -> String {
    env::var("SUDO_USER")
        .ok()
        .or_else(|| env::var("USER").ok())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "l400".to_string())
}

fn write_job_at(
    base: &Path,
    pid: u64,
    name: &str,
    user: &str,
    workload: WorkloadType,
    status: JobStatus,
    command: &str,
) -> Result<(), CgroupError> {
    let registry = job_registry_path(base);
    std::fs::create_dir_all(&registry)?;
    let payload = format!(
        "pid={pid}\nname={name}\nuser={user}\nworkload={}\nstatus={status}\nsubsystem={}\ncommand={command}\n",
        workload_name(workload),
        workload_name(workload)
    );
    std::fs::write(job_file(base, pid), payload)?;
    Ok(())
}

fn update_job_status_at(base: &Path, pid: u64, status: JobStatus) -> Result<(), CgroupError> {
    let path = job_file(base, pid);
    if !path.exists() {
        return Err(CgroupError::InvalidJob(format!("job {} not found", pid)));
    }
    let content = std::fs::read_to_string(&path)?;
    let mut updated = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.starts_with("status=") {
            lines.push(format!("status={status}"));
            updated = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !updated {
        lines.push(format!("status={status}"));
    }
    lines.push(String::new());
    std::fs::write(path, lines.join("\n"))?;
    Ok(())
}

fn parse_job(content: &str) -> Result<WorkloadJob, CgroupError> {
    let mut pid = None;
    let mut name = None;
    let mut user = None;
    let mut workload = None;
    let mut status = None;
    let mut subsystem = None;
    let mut command = None;

    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "pid" => pid = value.parse::<u64>().ok(),
                "name" => name = Some(value.to_string()),
                "user" => user = Some(value.to_string()),
                "workload" => workload = Some(workload_from_name(value)?),
                "status" => status = value.parse().ok(),
                "subsystem" => subsystem = Some(value.to_string()),
                "command" => command = Some(value.to_string()),
                _ => {}
            }
        }
    }

    Ok(WorkloadJob {
        pid: pid.ok_or_else(|| CgroupError::InvalidJob("missing pid".to_string()))?,
        name: name.ok_or_else(|| CgroupError::InvalidJob("missing name".to_string()))?,
        user: user.ok_or_else(|| CgroupError::InvalidJob("missing user".to_string()))?,
        workload: workload
            .ok_or_else(|| CgroupError::InvalidJob("missing workload".to_string()))?,
        status: status.ok_or_else(|| CgroupError::InvalidJob("missing status".to_string()))?,
        subsystem: subsystem.unwrap_or_else(|| "UNKNOWN".to_string()),
        command: command.unwrap_or_default(),
    })
}

fn list_jobs_at(base: &Path) -> Result<Vec<WorkloadJob>, CgroupError> {
    let registry = job_registry_path(base);
    if !registry.exists() {
        return Ok(Vec::new());
    }

    let mut jobs = Vec::new();
    for entry in std::fs::read_dir(&registry)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        let job = parse_job(&content)?;
        if PathBuf::from(format!("/proc/{}", job.pid)).exists() || job.status != JobStatus::Active {
            jobs.push(job);
        } else {
            let mut failed_job = job.clone();
            failed_job.status = JobStatus::Failed;
            let _ = update_job_status_at(base, failed_job.pid, JobStatus::Failed);
            jobs.push(failed_job);
        }
    }
    jobs.sort_by(|left, right| left.pid.cmp(&right.pid));
    Ok(jobs)
}

fn qinter_path() -> PathBuf {
    l400_root().join(QINTER_SLICE)
}

fn qbatch_path() -> PathBuf {
    l400_root().join(QBATCH_SLICE)
}

fn write_cgroup_param(path: &Path, file: &str, value: impl AsRef<str>) -> Result<(), CgroupError> {
    let file_path = path.join(file);
    std::fs::write(&file_path, value.as_ref())?;
    Ok(())
}

fn read_cgroup_param(path: &Path, file: &str) -> Result<String, CgroupError> {
    let file_path = path.join(file);
    let content = std::fs::read_to_string(&file_path)?;
    Ok(content.trim().to_string())
}

pub fn is_cgroup_v2_available() -> bool {
    std::path::PathBuf::from("/sys/fs/cgroup/cgroup.controllers").exists()
}

fn create_slice(name: &str, params: &CgroupParams) -> Result<PathBuf, CgroupError> {
    let slice_path = l400_root().join(name);

    if !slice_path.exists() {
        std::fs::create_dir_all(&slice_path)?;
    }

    write_cgroup_param(&slice_path, "cpu.weight", params.cpu_weight.to_string())?;
    write_cgroup_param(&slice_path, "cpu.max", &params.cpu_max)?;
    write_cgroup_param(&slice_path, "io.weight", params.io_weight.to_string())?;
    write_cgroup_param(&slice_path, "memory.high", &params.memory_high)?;
    write_cgroup_param(&slice_path, "memory.max", &params.memory_max)?;
    write_cgroup_param(&slice_path, "pids.max", params.pids_max.to_string())?;

    Ok(slice_path)
}

pub fn create_l400_slices() -> Result<(), CgroupError> {
    if !is_cgroup_v2_available() {
        return Err(CgroupError::NotAvailable);
    }

    if !l400_root().exists() {
        std::fs::create_dir_all(l400_root())?;
    }

    create_slice(QINTER_SLICE, &CgroupParams::interactive())?;
    create_slice(QBATCH_SLICE, &CgroupParams::batch())?;

    Ok(())
}

fn get_cgroup_path_for_pid(pid: u64) -> Result<PathBuf, CgroupError> {
    let cgroup_path = PathBuf::from(format!("/proc/{}/cgroup", pid));
    let content = std::fs::read_to_string(&cgroup_path)?;

    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 && parts[0] == "0" {
            return Ok(PathBuf::from(parts[2].trim()));
        }
    }

    Err(CgroupError::NotAvailable)
}

pub fn assign_to_workload(pid: u64, workload: WorkloadType) -> Result<(), CgroupError> {
    if !is_cgroup_v2_available() {
        return Err(CgroupError::NotAvailable);
    }

    let slice_path = match workload {
        WorkloadType::Interactive => qinter_path(),
        WorkloadType::Batch => qbatch_path(),
    };

    if !slice_path.exists() {
        create_l400_slices()?;
    }

    let tasks_file = slice_path.join("cgroup.threads");
    std::fs::write(&tasks_file, pid.to_string())?;

    Ok(())
}

pub fn register_job(
    pid: u64,
    name: &str,
    user: &str,
    workload: WorkloadType,
    status: JobStatus,
    command: &str,
) -> Result<(), CgroupError> {
    write_job_at(&l400_run_dir(), pid, name, user, workload, status, command)
}

pub fn register_current_job(
    name: &str,
    workload: WorkloadType,
    status: JobStatus,
    command: &str,
) -> Result<u64, CgroupError> {
    let pid = std::process::id() as u64;
    register_job(pid, name, &current_user_name(), workload, status, command)?;
    Ok(pid)
}

pub fn update_job_status(pid: u64, status: JobStatus) -> Result<(), CgroupError> {
    update_job_status_at(&l400_run_dir(), pid, status)
}

pub fn remove_job(pid: u64) -> Result<(), CgroupError> {
    let path = job_file(&l400_run_dir(), pid);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn list_jobs() -> Result<Vec<WorkloadJob>, CgroupError> {
    list_jobs_at(&l400_run_dir())
}

pub fn get_current_workload() -> Result<WorkloadType, CgroupError> {
    if !is_cgroup_v2_available() {
        return Err(CgroupError::NotAvailable);
    }

    let cgroup = get_cgroup_path_for_pid(std::process::id() as u64)?;

    let cgroup_str = cgroup.to_string_lossy();

    if cgroup_str.contains(QINTER_SLICE) {
        Ok(WorkloadType::Interactive)
    } else if cgroup_str.contains(QBATCH_SLICE) {
        Ok(WorkloadType::Batch)
    } else {
        Err(CgroupError::NotAvailable)
    }
}

pub fn set_cpu_priority(workload: WorkloadType, weight: u64) -> Result<(), CgroupError> {
    if !is_cgroup_v2_available() {
        return Err(CgroupError::NotAvailable);
    }

    let slice_path = match workload {
        WorkloadType::Interactive => qinter_path(),
        WorkloadType::Batch => qbatch_path(),
    };

    write_cgroup_param(&slice_path, "cpu.weight", weight.to_string())?;
    Ok(())
}

pub fn set_memory_limit(workload: WorkloadType, high: u64, max: u64) -> Result<(), CgroupError> {
    if !is_cgroup_v2_available() {
        return Err(CgroupError::NotAvailable);
    }

    let slice_path = match workload {
        WorkloadType::Interactive => qinter_path(),
        WorkloadType::Batch => qbatch_path(),
    };

    write_cgroup_param(&slice_path, "memory.high", high.to_string())?;
    write_cgroup_param(&slice_path, "memory.max", max.to_string())?;
    Ok(())
}

pub fn get_workload_params(workload: WorkloadType) -> Result<CgroupParams, CgroupError> {
    if !is_cgroup_v2_available() {
        return Err(CgroupError::NotAvailable);
    }

    let slice_path = match workload {
        WorkloadType::Interactive => qinter_path(),
        WorkloadType::Batch => qbatch_path(),
    };

    Ok(CgroupParams {
        cpu_weight: read_cgroup_param(&slice_path, "cpu.weight")?
            .parse()
            .unwrap_or(100),
        cpu_max: read_cgroup_param(&slice_path, "cpu.max")?,
        io_weight: read_cgroup_param(&slice_path, "io.weight")?
            .parse()
            .unwrap_or(100),
        memory_high: read_cgroup_param(&slice_path, "memory.high")?,
        memory_max: read_cgroup_param(&slice_path, "memory.max")?,
        pids_max: read_cgroup_param(&slice_path, "pids.max")?
            .parse()
            .unwrap_or(1024),
    })
}

pub fn cleanup_l400_slices() -> Result<(), CgroupError> {
    if !l400_root().exists() {
        return Ok(());
    }

    std::fs::remove_dir(qinter_path())?;
    std::fs::remove_dir(qbatch_path())?;

    if l400_root()
        .read_dir()
        .map(|mut d| d.next().is_none())
        .unwrap_or(true)
    {
        std::fs::remove_dir(l400_root())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cgroup_params_defaults() {
        let params = CgroupParams::default();
        assert_eq!(params.cpu_weight, 100);
        assert_eq!(params.io_weight, 100);
    }

    #[test]
    fn test_cgroup_params_interactive() {
        let params = CgroupParams::interactive();
        assert_eq!(params.cpu_weight, 10000);
        assert!(params.cpu_weight > CgroupParams::batch().cpu_weight);
    }

    #[test]
    fn test_cgroup_params_batch() {
        let params = CgroupParams::batch();
        assert_eq!(params.cpu_weight, 100);
        assert_eq!(params.io_weight, 50);
        assert!(params.pids_max > CgroupParams::interactive().pids_max);
    }

    #[test]
    fn test_slice_paths() {
        assert_eq!(
            qinter_path(),
            PathBuf::from("/sys/fs/cgroup/l400.slice/l400.qinter")
        );
        assert_eq!(
            qbatch_path(),
            PathBuf::from("/sys/fs/cgroup/l400.slice/l400.qbatch")
        );
    }

    #[test]
    fn test_cgroup_v2_detection() {
        let available = is_cgroup_v2_available();
        if available {
            assert!(
                l400_root().join("cgroup.controllers").exists()
                    || std::path::PathBuf::from("/sys/fs/cgroup/cgroup.controllers").exists()
            );
        }
    }

    #[test]
    fn test_workload_type_from_cgroup_current_process() {
        if !is_cgroup_v2_available() {
            return;
        }

        let result = get_current_workload();
        assert!(
            result.is_ok() || matches!(result, Err(CgroupError::NotAvailable)),
            "Should either get current workload or gracefully fail"
        );
    }

    #[test]
    fn test_job_registry_round_trip() {
        let root = tempdir().unwrap();
        let pid = std::process::id() as u64;
        write_job_at(
            root.path(),
            pid,
            "BATCHDEMO",
            "l400",
            WorkloadType::Batch,
            JobStatus::Active,
            "demo command",
        )
        .unwrap();

        let jobs = list_jobs_at(root.path()).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "BATCHDEMO");
        assert_eq!(jobs[0].subsystem, "QBATCH");

        update_job_status_at(root.path(), pid, JobStatus::Completed).unwrap();
        let jobs = list_jobs_at(root.path()).unwrap();
        assert_eq!(jobs[0].status, JobStatus::Completed);
    }
}
