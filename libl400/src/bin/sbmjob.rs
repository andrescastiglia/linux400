use clap::Parser;
use l400::cgroup::{
    assign_to_workload, job_log_path, register_job, update_job_status, JobStatus, WorkloadType,
};
use std::env;
use std::fs::OpenOptions;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
#[command(author, version, about = "Submit Job (SBMJOB) - Linux/400", long_about = None)]
struct Args {
    /// Comando a ejecutar
    #[arg(required = true)]
    cmd: String,

    /// Argumentos para el comando
    #[arg(trailing_var_arg = true)]
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
        let log_stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        let log_stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        let status = Command::new(&args.cmd)
            .args(&args.args)
            .stdout(log_stdout.map(Stdio::from).unwrap_or_else(Stdio::null))
            .stderr(log_stderr.map(Stdio::from).unwrap_or_else(Stdio::null))
            .status();

        let final_status = match status {
            Ok(s) if s.success() => JobStatus::Completed,
            _ => JobStatus::Failed,
        };

        // 4. Actualizar el estado final
        let _ = update_job_status(pid, final_status);
    } else {
        // Somos el SBMJOB original que invoca el usuario.
        // Hacemos fork/spawn de nosotros mismos con --daemon.

        #[allow(clippy::zombie_processes)]
        let child = Command::new(env::current_exe().unwrap())
            .arg(&args.cmd)
            .args(&args.args)
            .arg("--daemon")
            .arg("--job")
            .arg(&args.job)
            .arg("--jobq")
            .arg(&jobq)
            .arg("--user")
            .arg(&user)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            // Desvincular para que corra en background independientemente
            .process_group(0)
            .spawn()
            .expect("SBMJOB falló al inicializar el proceso batch");

        println!(
            "Trabajo {} enviado a la cola de trabajos {}. PID={}",
            args.job,
            jobq,
            child.id()
        );
    }
}
