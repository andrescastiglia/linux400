use clap::Parser;
use l400::cgroup::{
    JobStatus, WorkloadType, assign_to_workload, job_log_path, register_job, update_job_status,
};
use std::env;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about = "Submit Job (SBMJOB) - Linux/400", long_about = None)]
struct Args {
    /// Comando a ejecutar
    #[arg(required = true)]
    cmd: String,

    /// Argumentos para el comando
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,

    /// Ejecutar como daemon/hijo (uso interno)
    #[arg(long, hide = true)]
    daemon: bool,

    /// Nombre del trabajo (job name)
    #[arg(short, long, default_value = "QBATCH")]
    job: String,

    /// Cola de trabajos/subsistema
    #[arg(long, default_value = "QBATCH")]
    jobq: String,

    /// Usuario del trabajo (user)
    #[arg(short, long)]
    user: Option<String>,
}

fn current_user_name() -> String {
    env::var("SUDO_USER")
        .ok()
        .or_else(|| env::var("USER").ok())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "l400".to_string())
}

fn spool_file_path(pid: u64) -> PathBuf {
    env::var("L400_SPOOL_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| l400::resolve_l400_root().join("QUSRSYS").join("QSPL"))
        .join(format!("{}_{}.splf", pid, chrono_like_timestamp()))
}

fn chrono_like_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn append_line(path: &PathBuf, line: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn stream_to_spool_and_log<R>(
    mut reader: R,
    label: &'static str,
    spool_path: PathBuf,
    log_path: PathBuf,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut spool = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&spool_path)
            .ok();
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();

        if let Some(file) = spool.as_mut() {
            let _ = writeln!(file, "--- {label} ---");
        }
        if let Some(file) = log.as_mut() {
            let _ = writeln!(file, "--- {label} ---");
        }

        let mut buffer = [0_u8; 8192];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            if let Some(file) = spool.as_mut() {
                let _ = file.write_all(&buffer[..read]);
                let _ = file.flush();
            }
            if let Some(file) = log.as_mut() {
                let _ = file.write_all(&buffer[..read]);
                let _ = file.flush();
            }
        }
    })
}

fn main() {
    let args = Args::parse();
    let user = args.user.unwrap_or_else(current_user_name);
    let jobq = args.jobq.trim().to_uppercase();
    if jobq != "QBATCH" {
        eprintln!(
            "SBMJOB Error: JOBQ({}) no soportada; use JOBQ(QBATCH).",
            jobq
        );
        std::process::exit(2);
    }

    if args.daemon {
        // Somos el proceso daemon que maneja la ejecución real en QBATCH
        let pid = std::process::id() as u64;

        // 1. Asignar este daemon al cgroup QBATCH
        if let Err(e) = assign_to_workload(pid, WorkloadType::Batch) {
            eprintln!("SBMJOB Error: No se pudo asignar a QBATCH: {}", e);
            // Ignoramos el error para permitir ejecución fallback en sistemas sin cgroups
        }

        let cmd_str = format!("{} {}", args.cmd, args.args.join(" "));

        // 2. Registrar el trabajo en el Job Registry como JOBQ y luego ACTIVE.
        if let Err(e) = register_job(
            pid,
            &args.job,
            &user,
            WorkloadType::Batch,
            JobStatus::Active,
            &cmd_str,
        ) {
            eprintln!("SBMJOB Error: No se pudo registrar el job: {}", e);
        }
        let _ = update_job_status(pid, JobStatus::Active);

        // 3. Ejecutar el comando de usuario
        let log_path = job_log_path(pid);
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        let spool_path = spool_file_path(pid);
        if let Some(parent) = spool_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut spool = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&spool_path)
            .ok();
        if let Some(file) = spool.as_mut() {
            let _ = writeln!(
                file,
                "spool_version=1\njob={} pid={} user={} jobq={} command={} status=RUN submitted_at={}",
                args.job,
                pid,
                user,
                jobq,
                cmd_str,
                chrono_like_timestamp()
            );
        }
        if let Some(file) = log.as_mut() {
            let _ = writeln!(
                file,
                "job={} pid={} user={} jobq={} command={} status=RUN",
                args.job, pid, user, jobq, cmd_str
            );
        }

        drop(spool);
        drop(log);

        let child = Command::new(&args.cmd)
            .args(&args.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let final_status = match child {
            Ok(mut child) => {
                let stdout_handle = child.stdout.take().map(|stdout| {
                    stream_to_spool_and_log(stdout, "stdout", spool_path.clone(), log_path.clone())
                });
                let stderr_handle = child.stderr.take().map(|stderr| {
                    stream_to_spool_and_log(stderr, "stderr", spool_path.clone(), log_path.clone())
                });

                let status = child.wait();
                if let Some(handle) = stdout_handle {
                    let _ = handle.join();
                }
                if let Some(handle) = stderr_handle {
                    let _ = handle.join();
                }

                match status {
                    Ok(status) if status.success() => {
                        append_line(&spool_path, &format!("exit_status={status}"));
                        append_line(&log_path, &format!("exit_status={status}"));
                        JobStatus::Completed
                    }
                    Ok(status) => {
                        append_line(&spool_path, &format!("exit_status={status}"));
                        append_line(&log_path, &format!("exit_status={status}"));
                        JobStatus::Failed
                    }
                    Err(error) => {
                        append_line(&spool_path, &format!("wait_error={error}"));
                        append_line(&log_path, &format!("wait_error={error}"));
                        JobStatus::Failed
                    }
                }
            }
            Err(error) => {
                append_line(&spool_path, &format!("spawn_error={error}"));
                append_line(&log_path, &format!("spawn_error={error}"));
                JobStatus::Failed
            }
        };

        // 4. Actualizar el estado final
        let _ = update_job_status(pid, final_status);
        append_line(
            &spool_path,
            &format!(
                "job={} status={} ended_at={}",
                args.job,
                final_status,
                chrono_like_timestamp()
            ),
        );
        append_line(
            &log_path,
            &format!(
                "job={} status={} ended_at={}",
                args.job,
                final_status,
                chrono_like_timestamp()
            ),
        );
    } else {
        // Somos el SBMJOB original que invoca el usuario.
        // Hacemos fork/spawn de nosotros mismos con --daemon.

        #[allow(clippy::zombie_processes)]
        let child = Command::new(env::current_exe().unwrap())
            .arg("--daemon")
            .arg("--job")
            .arg(&args.job)
            .arg("--jobq")
            .arg(&jobq)
            .arg("--user")
            .arg(&user)
            .arg(&args.cmd)
            .args(&args.args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            // Desvincular para que corra en background independientemente
            .process_group(0)
            .spawn()
            .expect("SBMJOB falló al inicializar el proceso batch");
        let cmd_str = format!("{} {}", args.cmd, args.args.join(" "));
        let _ = register_job(
            child.id() as u64,
            &args.job,
            &user,
            WorkloadType::Batch,
            JobStatus::JobQ,
            &cmd_str,
        );

        println!(
            "Trabajo {} enviado a la cola de trabajos {}. PID={}",
            args.job,
            jobq,
            child.id()
        );
    }
}
