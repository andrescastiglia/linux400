use aya::{programs::Lsm, Ebpf};
use log::{info, warn};
use std::fs;
use std::path::PathBuf;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    info!("Iniciando Linux/400 BPF Loader (Requiere privilegios Root)...");

    // Limpieza de límites de memoria (memlock rlimit) necesario para BPF en kernels previos al cgroup-bpf limits.
    // Aunque en kernel >= 6.11 esto no es estrictamente imperativo, es una buena práctica.
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        warn!("Fallo al remover el limite de memlock, ret: {}", ret);
    }

    // Ruta heurística relativa (asumiendo ejecución desde la raíz del workspace L400)
    let bpf_path = PathBuf::from("../l400-ebpf/target/bpfel-unknown-none/release/l400-ebpf");
    if !bpf_path.exists() {
        return Err(anyhow::anyhow!(
            "Binario BPF no encontrado en {:?}. ¿Ejecutaste 'cargo build --target bpfel-unknown-none'?",
            bpf_path
        ));
    }

    let bpf_data = fs::read(&bpf_path)?;
    let mut bpf = Ebpf::load(&bpf_data)?;

    // Inicializar el logger subyacente para bpf (requiere aya_log que configuramos en l400-ebpf-common/Cargo.toml)
    if let Err(e) = aya_log::EbpfLogger::init(&mut bpf) {
        warn!("No se pudo inicializar el logger BPF trace: {}", e);
    }

    info!("Bytecode de eBPF cargado exitosamente al Kernel.");

    // Enganchar LSM hook "file_open"
    let program: &mut Lsm = bpf.program_mut("file_open").unwrap().try_into()?;
    program.load()?;
    program.attach()?;

    // Enganchar LSM hook "bprm_check_security"
    let program2: &mut Lsm = bpf.program_mut("bprm_check_security").unwrap().try_into()?;
    program2.load()?;
    program2.attach()?;

    info!("LSM Hooks 'file_open' y 'bprm_check_security' ensamblados y activados.");
    info!("La protección nativa OS/400 está en curso. (Presione Ctrl+C para salir)...");

    signal::ctrl_c().await?;
    info!("Señal capturada. Desprendiendo hooks BPF y saliendo...");

    Ok(())
}
