use anyhow::{Context, Result};
use aya::{programs::Lsm, Ebpf};
use clap::{Parser, ValueEnum};
use l400::{write_loader_status, LoaderStatus};
use log::{info, warn};
use std::fs;
use std::path::PathBuf;
use tokio::signal;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LoaderMode {
    Full,
    Degraded,
    Dev,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Linux/400 eBPF loader")]
struct Args {
    #[arg(long, env = "L400_LOADER_MODE", value_enum, default_value = "full")]
    mode: LoaderMode,
    #[arg(long)]
    once: bool,
}

struct LoaderRuntime {
    mode: LoaderMode,
    protection_active: bool,
    bpf: Option<Ebpf>,
    bpf_path: Option<PathBuf>,
}

impl LoaderMode {
    fn as_str(self) -> &'static str {
        match self {
            LoaderMode::Full => "full",
            LoaderMode::Degraded => "degraded",
            LoaderMode::Dev => "dev",
        }
    }
}

fn persist_status(runtime: &LoaderRuntime, phase: &str, last_error: Option<&str>) {
    let mut status = LoaderStatus::new(runtime.mode.as_str(), runtime.protection_active, phase);
    status.bpf_path = runtime
        .bpf_path
        .as_ref()
        .map(|path| path.display().to_string());
    status.last_error = last_error.map(|err| err.to_string());
    if let Err(err) = write_loader_status(&status) {
        warn!("No se pudo persistir loader-status: {}", err);
    }
}

fn resolve_bpf_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("L400_BPF_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    let candidates = [
        "/opt/l400/hooks/l400-ebpf",
        "/usr/lib/l400/hooks/l400-ebpf",
        "/l400/hooks/l400-ebpf",
        "../target/bpfel-unknown-none/release/l400-ebpf",
        "target/bpfel-unknown-none/release/l400-ebpf",
        "../l400-ebpf/target/bpfel-unknown-none/release/l400-ebpf",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow::anyhow!(
        "Binario BPF no encontrado. Configura L400_BPF_PATH o instala l400-ebpf en /opt/l400/hooks."
    ))
}

fn soft_fail(mode: LoaderMode, context: &str, err: anyhow::Error) -> Result<LoaderRuntime> {
    match mode {
        LoaderMode::Full => Err(err.context(context.to_string())),
        LoaderMode::Degraded => {
            warn!("{context}: {err}. Continuando en modo degradado sin protección activa.");
            let runtime = LoaderRuntime {
                mode,
                protection_active: false,
                bpf: None,
                bpf_path: None,
            };
            persist_status(&runtime, "fallback", Some(&format!("{context}: {err}")));
            Ok(runtime)
        }
        LoaderMode::Dev => {
            info!("{context}: {err}. Continuando en modo dev sin protección activa.");
            let runtime = LoaderRuntime {
                mode,
                protection_active: false,
                bpf: None,
                bpf_path: None,
            };
            persist_status(&runtime, "fallback", Some(&format!("{context}: {err}")));
            Ok(runtime)
        }
    }
}

fn print_mode_summary(runtime: &LoaderRuntime) {
    let protection = if runtime.protection_active {
        "active"
    } else {
        "inactive"
    };
    info!(
        "Modo del loader: {:?} (protección {})",
        runtime.mode, protection
    );
    match runtime.mode {
        LoaderMode::Full => {
            info!("Modo full: requiere cargar y adjuntar el hook eBPF o falla el arranque.");
        }
        LoaderMode::Degraded => {
            info!(
                "Modo degraded: intenta cargar el hook; si falla, el sistema sigue arriba sin enforcement."
            );
        }
        LoaderMode::Dev => {
            info!("Modo dev: prioriza feedback de desarrollo y tolera assets/BTF/hooks ausentes.");
        }
    }
}

fn init_loader(mode: LoaderMode) -> Result<LoaderRuntime> {
    // Limpieza de límites de memoria (memlock rlimit) necesario para BPF en kernels previos.
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        match mode {
            LoaderMode::Full => warn!("Fallo al remover el limite de memlock, ret: {}", ret),
            LoaderMode::Degraded | LoaderMode::Dev => {
                info!("memlock no pudo ajustarse (ret={}), continuando.", ret)
            }
        }
    }

    let bpf_path = match resolve_bpf_path() {
        Ok(path) => path,
        Err(err) => return soft_fail(mode, "No se pudo resolver el binario BPF", err),
    };

    let bpf_data = match fs::read(&bpf_path) {
        Ok(data) => data,
        Err(err) => {
            return soft_fail(
                mode,
                "No se pudo leer el bytecode eBPF",
                anyhow::Error::new(err),
            )
        }
    };

    let mut bpf = match Ebpf::load(&bpf_data) {
        Ok(bpf) => bpf,
        Err(err) => return soft_fail(mode, "No se pudo cargar el bytecode eBPF", err.into()),
    };

    if let Err(err) = aya_log::EbpfLogger::init(&mut bpf) {
        warn!("No se pudo inicializar el logger BPF trace: {}", err);
    }

    let btf = match aya::Btf::from_sys_fs() {
        Ok(btf) => btf,
        Err(err) => return soft_fail(mode, "No se pudo leer BTF del sistema", err.into()),
    };

    let program: &mut Lsm = match bpf.program_mut("file_open") {
        Some(program) => program.try_into().context("Programa file_open inválido")?,
        None => {
            return soft_fail(
                mode,
                "No existe el programa file_open",
                anyhow::anyhow!("missing file_open"),
            )
        }
    };
    if let Err(err) = program
        .load("file_open", &btf)
        .and_then(|_| program.attach())
    {
        return soft_fail(mode, "No se pudo adjuntar file_open", err.into());
    }

    let program2: &mut Lsm = match bpf.program_mut("bprm_check_security") {
        Some(program) => program
            .try_into()
            .context("Programa bprm_check_security inválido")?,
        None => {
            return soft_fail(
                mode,
                "No existe el programa bprm_check_security",
                anyhow::anyhow!("missing bprm_check_security"),
            )
        }
    };
    if let Err(err) = program2
        .load("bprm_check_security", &btf)
        .and_then(|_| program2.attach())
    {
        return soft_fail(mode, "No se pudo adjuntar bprm_check_security", err.into());
    }

    info!("LSM Hooks 'file_open' y 'bprm_check_security' ensamblados y activados.");
    let runtime = LoaderRuntime {
        mode,
        protection_active: true,
        bpf: Some(bpf),
        bpf_path: Some(bpf_path),
    };
    persist_status(&runtime, "active", None);
    Ok(runtime)
}

fn log_stats(runtime: &mut LoaderRuntime) -> Result<()> {
    if !runtime.protection_active {
        info!("Protección eBPF inactiva en este modo.");
        return Ok(());
    }

    let bpf = runtime
        .bpf
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("runtime inconsistente: no hay estado BPF"))?;
    let stats_map: aya::maps::HashMap<_, u32, u64> = aya::maps::HashMap::try_from(
        bpf.map_mut("L400_STATS")
            .context("mapa L400_STATS ausente")?,
    )?;

    let allowed = stats_map.get(&0, 0).unwrap_or(0);
    let denied = stats_map.get(&1, 0).unwrap_or(0);

    info!("--- Estadísticas de L400 ---");
    info!("Accesos Permitidos : {}", allowed);
    info!("Accesos Denegados  : {}", denied);

    for (i, obj) in l400_ebpf_common::VALID_OBJ_TYPES.iter().enumerate() {
        let count = stats_map.get(&(i as u32 + 2), 0).unwrap_or(0);
        if count > 0 {
            info!("  -> {} accesos a {}", count, obj.name);
        }
    }
    info!("----------------------------");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    info!("Iniciando Linux/400 BPF Loader...");
    let bootstrap = LoaderRuntime {
        mode: args.mode,
        protection_active: false,
        bpf: None,
        bpf_path: None,
    };
    persist_status(&bootstrap, "starting", None);

    let mut runtime = init_loader(args.mode)?;
    print_mode_summary(&runtime);

    if args.once {
        if runtime.protection_active {
            let _ = log_stats(&mut runtime);
        }
        return Ok(());
    }

    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                let _ = log_stats(&mut runtime);
            }
            _ = signal::ctrl_c() => {
                info!("Señal capturada. Desprendiendo hooks BPF y saliendo...");
                persist_status(&runtime, "stopped", None);
                break;
            }
        }
    }

    Ok(())
}
