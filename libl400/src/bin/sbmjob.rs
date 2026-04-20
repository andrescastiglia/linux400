use clap::Parser;
use l400::cgroup::{assign_to_workload, register_job, update_job_status, JobStatus, WorkloadType};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::env;

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

    if args.daemon {
        // Somos el proceso daemon que maneja la ejecución real en QBATCH
        let pid = std::process::id() as u64;

        // 1. Asignar este daemon al cgroup QBATCH
        if let Err(e) = assign_to_workload(pid, WorkloadType::Batch) {
            eprintln!("SBMJOB Error: No se pudo asignar a QBATCH: {}", e);
            // Ignoramos el error para permitir ejecución fallback en sistemas sin cgroups
        }

        let cmd_str = format!("{} {}", args.cmd, args.args.join(" "));

        // 2. Registrar el trabajo en el Job Registry como Active
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

        // 3. Ejecutar el comando de usuario
        let status = Command::new(&args.cmd)
            .args(&args.args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
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
        
        let child = Command::new(env::current_exe().unwrap())
            .arg(&args.cmd)
            .args(&args.args)
            .arg("--daemon")
            .arg("--job")
            .arg(&args.job)
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
            "Trabajo {} enviado a la cola de trabajos QBATCH. PID={}",
            args.job,
            child.id()
        );
    }
}
