use l400::{
    assign_to_workload, create_l400_slices, list_jobs, register_current_job, register_job,
    remove_job, update_job_status, WorkloadType,
};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn current_user() -> String {
    std::env::var("SUDO_USER")
        .ok()
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "l400".to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = create_l400_slices();
    let _ = assign_to_workload(std::process::id() as u64, WorkloadType::Interactive);
    let interactive_pid =
        register_current_job("WORKLOADDEMO", WorkloadType::Interactive, "ACTIVE", "workload_demo")?;

    let child = Command::new("sh")
        .arg("-lc")
        .arg("printf 'Linux/400 batch demo\\n'; sleep 1")
        .stdout(Stdio::piped())
        .spawn()?;

    let batch_pid = child.id() as u64;
    let batch_status = if assign_to_workload(batch_pid, WorkloadType::Batch).is_ok() {
        "ACTIVE"
    } else {
        "DEGRADED"
    };
    register_job(
        batch_pid,
        "BATCHDEMO",
        &current_user(),
        WorkloadType::Batch,
        batch_status,
        "sh -lc 'printf Linux/400 batch demo; sleep 1'",
    )?;

    thread::sleep(Duration::from_millis(100));
    println!("== Workload snapshot ==");
    for job in list_jobs()? {
        println!(
            "{} {} {} {} {}",
            job.pid, job.name, job.user, job.status, job.subsystem
        );
    }

    let output = child.wait_with_output()?;
    update_job_status(batch_pid, "COMPLETED")?;
    println!("== Batch output ==");
    print!("{}", String::from_utf8_lossy(&output.stdout));

    let _ = remove_job(batch_pid);
    let _ = update_job_status(interactive_pid, "COMPLETED");
    let _ = remove_job(interactive_pid);
    Ok(())
}
