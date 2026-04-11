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
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkloadType {
    Interactive,
    Batch,
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
}
