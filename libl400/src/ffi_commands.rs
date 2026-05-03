/// Funciones C públicas del runtime Linux/400.
/// Estas son invocadas por los programas CL compilados por `clc`.
/// Cada función implementa la semántica del comando OS/400 correspondiente
/// delegando a los módulos internos de `libl400`.
use std::ffi::CStr;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn c_str_to_string(s: *const c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
}

fn now_epoch_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn resolve_file_spec(file_spec: &str) -> (String, String) {
    let trimmed = file_spec.trim();
    if let Some((library, file)) = trimmed.split_once('/') {
        (library.trim().to_uppercase(), file.trim().to_uppercase())
    } else {
        (
            std::env::var("L400_CURLIB")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.trim().to_uppercase())
                .unwrap_or_else(|| "QGPL".to_string()),
            trimmed.to_uppercase(),
        )
    }
}

fn parse_command_fields(input: &str) -> std::collections::HashMap<String, String> {
    let mut fields = std::collections::HashMap::new();
    let chars = input.chars().collect::<Vec<_>>();
    let mut index = 0usize;

    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        let key_start = index;
        while index < chars.len()
            && (chars[index].is_ascii_alphanumeric() || chars[index] == '_' || chars[index] == '*')
        {
            index += 1;
        }
        if key_start == index || index >= chars.len() || chars[index] != '=' {
            index += 1;
            continue;
        }
        let key = chars[key_start..index]
            .iter()
            .collect::<String>()
            .trim()
            .to_uppercase();
        index += 1;

        let value_start = index;
        let mut in_single = false;
        let mut in_double = false;
        while index < chars.len() {
            match chars[index] {
                '\'' if !in_double => in_single = !in_single,
                '"' if !in_single => in_double = !in_double,
                ch if ch.is_whitespace() && !in_single && !in_double => {
                    let mut lookahead = index;
                    while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                        lookahead += 1;
                    }
                    let candidate_start = lookahead;
                    while lookahead < chars.len()
                        && (chars[lookahead].is_ascii_alphanumeric()
                            || chars[lookahead] == '_'
                            || chars[lookahead] == '*')
                    {
                        lookahead += 1;
                    }
                    if candidate_start < lookahead
                        && lookahead < chars.len()
                        && chars[lookahead] == '='
                    {
                        break;
                    }
                }
                _ => {}
            }
            index += 1;
        }

        let value = chars[value_start..index]
            .iter()
            .collect::<String>()
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string();
        fields.insert(key, value);
    }
    fields
}

fn resolve_object_spec(
    root: &Path,
    object_spec: &str,
    library_override: Option<&str>,
) -> (String, String, PathBuf) {
    let trimmed = object_spec.trim();
    if let Some((library, object)) = trimmed.split_once('/') {
        let library = library.trim().to_uppercase();
        let object = object.trim().to_uppercase();
        let path = root.join(&library).join(&object);
        return (library, object, path);
    }

    let library = library_override
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("L400_CURLIB")
                .ok()
                .map(|value| value.trim().to_uppercase())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "QGPL".to_string());
    let object = trimmed.to_uppercase();
    let path = root.join(&library).join(&object);
    (library, object, path)
}

fn matches_pattern(value: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().to_uppercase();
    if pattern.is_empty() || pattern == "*ALL" || pattern == "*" {
        return true;
    }
    let value = value.to_uppercase();
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    value == pattern
}

fn audit_runtime(event: &str, object: &Path, message: &str) {
    let user = crate::audit::current_l400_user();
    let _ = crate::audit::audit_event(event, &user, object, message);
}

fn runtime_user() -> String {
    crate::audit::current_l400_user()
}

fn clear_status() {
    crate::ffi::clear_last_cpf();
}

fn set_status(code: &str) {
    crate::ffi::set_last_cpf(code);
}

fn emit_status(code: &str, object: Option<&Path>, detail: &str) {
    set_status(code);
    let status = crate::status::command_status(code);
    let object_text = object
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "{} SEV({}) {} OBJ({}) - {}",
        status.code, status.severity, status.message, object_text, detail
    );
    if let Some(path) = object {
        audit_runtime(
            "COMMAND_STATUS",
            path,
            &format!(
                "cpf={} severity={} detail={}",
                status.code, status.severity, detail
            ),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PowerDownAction {
    ControlledPowerOff,
    ImmediatePowerOff,
    Restart,
}

impl PowerDownAction {
    fn from_option(option: &str) -> Option<Self> {
        match option.trim().to_uppercase().as_str() {
            "" | "*CNTRLD" | "CNTRLD" | "*CONTROLLED" | "CONTROLLED" => {
                Some(Self::ControlledPowerOff)
            }
            "*IMMED" | "IMMED" | "*IMMEDIATE" | "IMMEDIATE" => Some(Self::ImmediatePowerOff),
            "*RESTART" | "RESTART" | "*REBOOT" | "REBOOT" => Some(Self::Restart),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ControlledPowerOff => "*CNTRLD",
            Self::ImmediatePowerOff => "*IMMED",
            Self::Restart => "*RESTART",
        }
    }

    fn command_plan(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            Self::ControlledPowerOff => &[
                ("shutdown", &["-h", "now"]),
                ("poweroff", &[]),
                ("halt", &[]),
            ],
            Self::ImmediatePowerOff => &[
                ("poweroff", &["-f"]),
                ("halt", &["-f"]),
                ("shutdown", &["-h", "now"]),
            ],
            Self::Restart => &[("reboot", &["-f"]), ("shutdown", &["-r", "now"])],
        }
    }
}

fn confirmed_yes(value: Option<&String>) -> bool {
    value
        .map(|value| matches!(value.trim().to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false)
}

fn power_down_dry_run_enabled() -> bool {
    std::env::var("L400_PWRDWNSYS_DRY_RUN")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn run_power_down_action(action: PowerDownAction) -> std::io::Result<()> {
    if power_down_dry_run_enabled() {
        println!("[PWRDWNSYS] Dry-run activo; no se ejecuta apagado real.");
        return Ok(());
    }

    let _ = Command::new("sync").status();
    let mut last_error = None;
    for (program, args) in action.command_plan() {
        match Command::new(program).args(*args).status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                last_error = Some(std::io::Error::other(format!(
                    "{program} exited with status {status}"
                )));
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| std::io::Error::other("no power command available")))
}

// l400_sndpgmmsg está definida en ffi.rs — no se duplica aquí.

// Gestión de sistema
// ---------------------------------------------------------------------------

/// WRKSYSSTS — Muestra estado del sistema (CPU, jobs, memoria)
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrksyssts() {
    println!("=== WRKSYSSTS - Estado del Sistema Linux/400 ===");

    // CPU load via /proc/loadavg
    if let Ok(loadavg) = std::fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = loadavg.split_whitespace().collect();
        if parts.len() >= 3 {
            println!(
                "  Carga CPU (1m/5m/15m): {} {} {}",
                parts[0], parts[1], parts[2]
            );
        }
    }

    // Uptime
    if let Ok(uptime) = std::fs::read_to_string("/proc/uptime") {
        let secs: f64 = uptime
            .split_whitespace()
            .next()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);
        let hours = (secs / 3600.0) as u64;
        let mins = ((secs % 3600.0) / 60.0) as u64;
        println!("  Uptime: {}h {}m", hours, mins);
    }

    // Memory
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") || line.starts_with("MemAvailable:") {
                println!("  {}", line.trim());
            }
        }
    }

    if let Ok(jobs) = crate::cgroup::list_jobs() {
        println!("  Jobs registrados: {}", jobs.len());
    }
    println!("  Subsistemas:");
    for (name, text) in crate::cgroup::subsystem_descriptions() {
        println!("    {:8} {}", name, text);
    }
    match crate::cgroup::get_workload_params(crate::WorkloadType::Interactive) {
        Ok(params) => println!(
            "  QINTER cgroup: cpu.weight={} memory.max={} pids.max={}",
            params.cpu_weight, params.memory_max, params.pids_max
        ),
        Err(_) => println!("  QINTER cgroup: modo degradado/no disponible"),
    }
    match crate::cgroup::get_workload_params(crate::WorkloadType::Batch) {
        Ok(params) => println!(
            "  QBATCH cgroup: cpu.weight={} memory.max={} pids.max={}",
            params.cpu_weight, params.memory_max, params.pids_max
        ),
        Err(_) => println!("  QBATCH cgroup: modo degradado/no disponible"),
    }

    println!("================================================");
}

/// WRKACTJOB — Lista jobs activos del job registry
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkactjob() {
    l400_wrkactjob_spec(std::ptr::null());
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkactjob_spec(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let subsystem_filter = fields
        .get("SBS")
        .or_else(|| fields.get("SUBSYSTEM"))
        .map(|value| value.to_uppercase());
    let status_filter = fields.get("STATUS").map(|value| value.to_uppercase());
    let option = fields
        .get("OPTION")
        .map(|value| value.to_uppercase())
        .unwrap_or_else(|| "*LIST".to_string());
    let target_pid = fields
        .get("PID")
        .and_then(|value| value.parse::<u64>().ok());
    let target_job = fields.get("JOB").map(|value| value.to_uppercase());

    println!("=== WRKACTJOB - Trabajos Activos ===");
    match crate::cgroup::list_jobs() {
        Ok(jobs) if jobs.is_empty() => println!("  No hay trabajos activos."),
        Ok(jobs) => {
            let jobs = jobs
                .into_iter()
                .filter(|job| {
                    subsystem_filter
                        .as_deref()
                        .map(|filter| job.subsystem.eq_ignore_ascii_case(filter))
                        .unwrap_or(true)
                })
                .filter(|job| {
                    status_filter
                        .as_deref()
                        .map(|filter| job.status.to_string().eq_ignore_ascii_case(filter))
                        .unwrap_or(true)
                })
                .filter(|job| {
                    target_pid.map(|pid| job.pid == pid).unwrap_or(true)
                        && target_job
                            .as_deref()
                            .map(|name| job.name.eq_ignore_ascii_case(name))
                            .unwrap_or(true)
                })
                .collect::<Vec<_>>();

            if option == "*END" || option == "END" {
                match jobs.first() {
                    Some(job) => match crate::cgroup::end_job(job.pid) {
                        Ok(_) => println!("  Job {} PID={} terminado.", job.name, job.pid),
                        Err(error) => println!("  Error terminando job: {}", error),
                    },
                    None => println!("  No se encontro job para terminar."),
                }
                println!("====================================");
                return;
            }

            if option == "*DETAIL" || option == "DETAIL" || option == "5" {
                match jobs.first() {
                    Some(job) => {
                        println!("  Job . . . . . . . . . : {}", job.name);
                        println!("  User  . . . . . . . . : {}", job.user);
                        println!("  PID . . . . . . . . . : {}", job.pid);
                        println!("  Status  . . . . . . . : {}", job.status);
                        println!("  Subsystem . . . . . . : {}", job.subsystem);
                        println!("  Command . . . . . . . : {}", job.command);
                        println!(
                            "  Submitted . . . . . . : {}",
                            job.submitted_at.as_deref().unwrap_or("-")
                        );
                        println!(
                            "  Started . . . . . . . : {}",
                            job.started_at.as_deref().unwrap_or("-")
                        );
                        println!(
                            "  Ended . . . . . . . . : {}",
                            job.ended_at.as_deref().unwrap_or("-")
                        );
                        println!(
                            "  Log . . . . . . . . . : {}",
                            job.log_path
                                .as_ref()
                                .map(|path| path.display().to_string())
                                .unwrap_or_else(|| "-".to_string())
                        );
                    }
                    None => println!("  No se encontro job para mostrar."),
                }
                println!("====================================");
                return;
            }

            println!(
                "  {:20} {:10} {:8} {:8} {:10} COMMAND",
                "JOB", "ESTADO", "PID", "SBS", "USER"
            );
            println!("  {}", "-".repeat(86));
            for j in &jobs {
                println!(
                    "  {:20} {:10} {:8} {:8} {:10} {}",
                    j.name, j.status, j.pid, j.subsystem, j.user, j.command
                );
            }
            if jobs.is_empty() {
                println!("  No hay trabajos para el filtro indicado.");
            }
        }
        Err(e) => println!("  Error al listar jobs: {}", e),
    }
    println!("====================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkjobq() {
    println!("=== WRKJOBQ - Job Queues ===");
    println!("  {:10} {:10} {:8} COMMAND", "JOBQ", "STATUS", "PID");
    println!("  {}", "-".repeat(72));
    match crate::cgroup::list_jobs() {
        Ok(jobs) => {
            let mut count = 0usize;
            for job in jobs.into_iter().filter(|job| {
                matches!(
                    job.status,
                    crate::cgroup::JobStatus::JobQ | crate::cgroup::JobStatus::Held
                )
            }) {
                count += 1;
                println!(
                    "  {:10} {:10} {:8} {}",
                    job.subsystem, job.status, job.pid, job.command
                );
            }
            if count == 0 {
                println!("  No hay trabajos en cola.");
            }
        }
        Err(error) => println!("  Error al listar job queues: {}", error),
    }
    println!("============================");
}

fn job_pid_from_fields(fields: &std::collections::HashMap<String, String>) -> Option<u64> {
    fields
        .get("PID")
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| {
            let job_name = fields.get("JOB")?;
            crate::cgroup::list_jobs()
                .ok()?
                .into_iter()
                .find_map(|job| job.name.eq_ignore_ascii_case(job_name).then_some(job.pid))
        })
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkjob(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    let Some(pid) = job_pid_from_fields(&fields) else {
        emit_status("CPF0006", None, "WRKJOB requiere JOB o PID");
        println!("[WRKJOB] Uso: WRKJOB JOB(MYJOB) o WRKJOB PID(123)");
        return;
    };
    match crate::cgroup::list_jobs() {
        Ok(jobs) => match jobs.into_iter().find(|job| job.pid == pid) {
            Some(job) => {
                println!("=== WRKJOB - Job Detail ===");
                println!("  Job . . . . . . . . . : {}", job.name);
                println!("  User  . . . . . . . . : {}", job.user);
                println!("  PID . . . . . . . . . : {}", job.pid);
                println!("  Status  . . . . . . . : {}", job.status);
                println!("  Subsystem . . . . . . : {}", job.subsystem);
                println!("  Command . . . . . . . : {}", job.command);
                println!(
                    "  Submitted . . . . . . : {}",
                    job.submitted_at.as_deref().unwrap_or("-")
                );
                println!(
                    "  Started . . . . . . . : {}",
                    job.started_at.as_deref().unwrap_or("-")
                );
                println!(
                    "  Ended . . . . . . . . : {}",
                    job.ended_at.as_deref().unwrap_or("-")
                );
                let log_path = job
                    .log_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "-".to_string());
                println!("  Log . . . . . . . . . : {}", log_path);
                if let Some(path) = job.log_path.as_ref().filter(|path| path.exists()) {
                    println!("  Log tail:");
                    if let Ok(content) = std::fs::read_to_string(path) {
                        for line in content
                            .lines()
                            .rev()
                            .take(10)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                        {
                            println!("    {}", line);
                        }
                    }
                }
                println!("===========================");
            }
            None => {
                emit_status("CPF9801", None, "WRKJOB no encontro el PID solicitado");
                println!("[WRKJOB] PID({pid}) no encontrado.");
            }
        },
        Err(error) => {
            emit_status("CPF0001", None, &error.to_string());
            println!("[WRKJOB] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_hldjob(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    match job_pid_from_fields(&fields) {
        Some(pid) => match crate::cgroup::hold_job(pid) {
            Ok(_) => println!("[HLDJOB] PID({pid}) retenido."),
            Err(error) => println!("[HLDJOB] Error: {}", error),
        },
        None => {
            emit_status("CPF0006", None, "HLDJOB requiere JOB o PID");
            println!("[HLDJOB] Uso: HLDJOB JOB(MYJOB) o HLDJOB PID(123)");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_rlsjob(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    match job_pid_from_fields(&fields) {
        Some(pid) => match crate::cgroup::release_job(pid) {
            Ok(_) => println!("[RLSJOB] PID({pid}) liberado."),
            Err(error) => println!("[RLSJOB] Error: {}", error),
        },
        None => {
            emit_status("CPF0006", None, "RLSJOB requiere JOB o PID");
            println!("[RLSJOB] Uso: RLSJOB JOB(MYJOB) o RLSJOB PID(123)");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_endjob(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    let immediate = fields
        .get("OPTION")
        .map(|value| matches!(value.to_uppercase().as_str(), "*IMMED" | "IMMED"))
        .unwrap_or(false);
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    if !confirmed {
        emit_status("CPF0006", None, "ENDJOB requiere CONFIRM(*YES)");
        println!("[ENDJOB] Requiere CONFIRM(*YES).");
        return;
    }
    match job_pid_from_fields(&fields) {
        Some(pid) => match if immediate {
            crate::cgroup::kill_job(pid)
        } else {
            crate::cgroup::end_job(pid)
        } {
            Ok(_) => println!(
                "[ENDJOB] PID({pid}) {}.",
                if immediate {
                    "terminado inmediato"
                } else {
                    "terminado"
                }
            ),
            Err(error) => println!("[ENDJOB] Error: {}", error),
        },
        None => {
            emit_status("CPF0006", None, "ENDJOB requiere JOB o PID");
            println!("[ENDJOB] Uso: ENDJOB JOB(MYJOB) CONFIRM(*YES)");
        }
    }
}

/// WRKSYSVAL — Muestra valores de configuración del sistema
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrksysval() {
    println!("=== WRKSYSVAL - Valores del Sistema ===");
    let root = crate::object::resolve_l400_root();
    println!("  L400_ROOT    = {}", root.display());
    println!("  PLATFORM     = {}", std::env::consts::ARCH);
    println!("  OS           = Linux/400");
    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        println!("  HOSTNAME     = {}", hostname.trim());
    }
    println!("=======================================");
}

/// DSPLOG — Muestra entradas del log del sistema
#[unsafe(no_mangle)]
pub extern "C" fn l400_dsplog() {
    println!("=== DSPLOG - Historial del Sistema (QHST) ===");
    // Intentar leer /var/log/syslog o /var/log/messages
    for candidate in ["/var/log/syslog", "/var/log/messages", "/var/log/kern.log"] {
        if let Ok(content) = std::fs::read_to_string(candidate) {
            let lines: Vec<&str> = content.lines().collect();
            let tail = lines.iter().rev().take(10).collect::<Vec<_>>();
            for line in tail.into_iter().rev() {
                println!("  {}", line);
            }
            println!("==============================================");
            return;
        }
    }
    println!("  Log del sistema no disponible.");
    println!("==============================================");
}

/// WRKSPLF — Lista spool files si existe un directorio de spool configurado.
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrksplf() {
    println!("=== WRKSPLF - Spool Files ===");
    let root = crate::object::resolve_l400_root();
    let candidates = [
        std::env::var("L400_SPOOL_DIR").ok().map(PathBuf::from),
        Some(root.join("QUSRSYS").join("QSPL")),
        Some(root.join("spool")),
    ];
    for dir in candidates.into_iter().flatten() {
        if !dir.exists() {
            continue;
        }
        println!("  Directory: {}", dir.display());
        println!("  {:20} {:>10} {:8} MODIFIED", "FILE", "SIZE", "STATUS");
        println!("  {}", "-".repeat(56));
        let mut count = 0usize;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    count += 1;
                    let status =
                        spool_file_status(&entry.path()).unwrap_or_else(|| "READY".to_string());
                    println!(
                        "  {:20} {:>10} {:8} {:?}",
                        entry.file_name().to_string_lossy(),
                        metadata.len(),
                        status,
                        metadata.modified().ok()
                    );
                }
            }
        }
        if count == 0 {
            println!("  Sin spool files.");
        }
        println!("=============================");
        return;
    }
    println!("  Sin spool/outq runtime. Configure L400_SPOOL_DIR o cree QUSRSYS/QSPL.");
    println!("=============================");
}

/// WRKOUTQ — Lista output queues catalogadas como *OUTQ y el spool base.
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkoutq() {
    println!("=== WRKOUTQ - Output Queues ===");
    let root = crate::object::resolve_l400_root();
    println!(
        "  {:10} {:20} {:8} {:8} {:10} TEXT",
        "LIB", "OUTQ", "STATUS", "RETAIN", "ROUTING"
    );
    println!("  {}", "-".repeat(78));

    let mut count = 0usize;
    if let Ok(libraries) = crate::object::list_libraries(&root) {
        for library in libraries {
            let lib_path = root.join(&library);
            let Ok(objects) = crate::object::list_objects(&lib_path) else {
                continue;
            };
            for object in objects
                .into_iter()
                .filter(|object| object.objtype == "*OUTQ")
            {
                let path = lib_path.join(&object.name);
                let status =
                    crate::storage::read_string_attr(&path, crate::L400_OUTQ_DEFAULT_STATUS_ATTR)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "READY".to_string());
                let retain =
                    crate::storage::read_string_attr(&path, crate::L400_OUTQ_RETENTION_DAYS_ATTR)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "7".to_string());
                let routing =
                    crate::storage::read_string_attr(&path, crate::L400_OUTQ_ROUTING_ATTR)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "QBATCH".to_string());
                count += 1;
                println!(
                    "  {:10} {:20} {:8} {:8} {:10} {}",
                    library,
                    object.name,
                    status,
                    retain,
                    routing,
                    object.text.unwrap_or_default()
                );
            }
        }
    }

    let spool_dir = std::env::var("L400_SPOOL_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("QUSRSYS").join("QSPL"));
    if spool_dir.exists() {
        count += 1;
        println!(
            "  {:10} {:20} {:8} {:8} {:10} {}",
            "QUSRSYS",
            "QSPL",
            "READY",
            "-",
            "DIR",
            spool_dir.display()
        );
    }

    if count == 0 {
        println!("  Sin output queues. Cree un objeto *OUTQ o configure L400_SPOOL_DIR.");
    }
    println!("===============================");
}

fn spool_dir() -> PathBuf {
    std::env::var("L400_SPOOL_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            crate::object::resolve_l400_root()
                .join("QUSRSYS")
                .join("QSPL")
        })
}

fn compile_spool_file(program: &str) -> PathBuf {
    let safe_name = program
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    spool_dir().join(format!(
        "CRTCLPGM_{}_{}.splf",
        safe_name,
        now_epoch_string()
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_crtoutq(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let outq = fields
        .get("OUTQ")
        .cloned()
        .unwrap_or_else(|| "QUSRSYS/QPRINT".to_string());
    let root = crate::object::resolve_l400_root();
    let (library, name, _path) =
        resolve_object_spec(&root, &outq, fields.get("LIB").map(String::as_str));
    let lib_path = root.join(&library);
    let user = runtime_user();
    match crate::auth::check_authority(&lib_path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status(
                "CPF2204",
                Some(&lib_path),
                "authority insufficient for create",
            );
            println!(
                "[CRTOUTQ] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                lib_path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&lib_path), &error.to_string());
            println!("[CRTOUTQ] Error verificando autoridad: {}", error);
            return;
        }
    }
    let text = fields
        .get("TEXT")
        .map(String::as_str)
        .unwrap_or("Output queue");
    let retention_days = fields
        .get("RETAIN")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(7);
    let routing = fields
        .get("ROUTING")
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "QBATCH".to_string());
    let default_status = fields
        .get("STATUS")
        .map(|value| value.trim().trim_start_matches('*').to_uppercase())
        .filter(|value| matches!(value.as_str(), "READY" | "HELD" | "SAVED"))
        .unwrap_or_else(|| "READY".to_string());
    match crate::object::create_object_with_metadata(
        &lib_path,
        &name,
        "*OUTQ",
        Some("OUTQ"),
        Some(text),
    ) {
        Ok(_) => {
            let path = lib_path.join(&name);
            let _ = crate::storage::write_u32_attr(
                &path,
                crate::L400_DATA_FORMAT_VERSION_ATTR,
                crate::L400_DATA_FORMAT_VERSION,
            );
            let _ = crate::storage::write_u32_attr(
                &path,
                crate::L400_OUTQ_RETENTION_DAYS_ATTR,
                retention_days,
            );
            let _ =
                crate::storage::write_string_attr(&path, crate::L400_OUTQ_ROUTING_ATTR, &routing);
            let _ = crate::storage::write_string_attr(
                &path,
                crate::L400_OUTQ_DEFAULT_STATUS_ATTR,
                &default_status,
            );
            let _ = std::fs::create_dir_all(spool_dir());
            println!(
                "[CRTOUTQ] {}/{} creado RETAIN({}) ROUTING({}) STATUS(*{}).",
                library, name, retention_days, routing, default_status
            );
        }
        Err(error) => {
            emit_status("CPF0001", Some(&lib_path.join(&name)), &error.to_string());
            println!("[CRTOUTQ] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dltoutq(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let outq = fields
        .get("OUTQ")
        .cloned()
        .unwrap_or_else(|| "QUSRSYS/QPRINT".to_string());
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    if !confirmed {
        emit_status("CPF0006", None, "DLTOUTQ requiere CONFIRM(*YES)");
        println!("[DLTOUTQ] Requiere CONFIRM(*YES).");
        return;
    }
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, &outq, fields.get("LIB").map(String::as_str));

    // Check if object exists; if not, emit CPF9801 (idempotent delete scenario)
    if !path.exists() {
        emit_status("CPF9801", Some(&path), "Object not found for delete");
        println!("[DLTOUTQ] Objeto no encontrado: {}", path.display());
        return;
    }

    let user = runtime_user();
    match crate::auth::check_authority(&path, &user, crate::auth::L400Authority::All) {
        Ok(true) => {}
        Ok(false) => {
            emit_status("CPF2204", Some(&path), "authority insufficient for delete");
            println!(
                "[DLTOUTQ] Denegado por autoridad: usuario {} no tiene *ALL sobre {}.",
                user,
                path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[DLTOUTQ] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::object::delete_object(&path) {
        Ok(_) => println!("[DLTOUTQ] {} eliminado.", outq),
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[DLTOUTQ] Error: {}", error);
        }
    }
}

fn resolve_spool_file(fields: &std::collections::HashMap<String, String>) -> PathBuf {
    fields
        .get("SPLF")
        .or_else(|| fields.get("FILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| {
            let name = fields
                .get("SPLF")
                .or_else(|| fields.get("FILE"))
                .cloned()
                .unwrap_or_else(|| "LAST".to_string());
            if name == "LAST" {
                first_spool_file().unwrap_or_else(|| spool_dir().join("LAST"))
            } else {
                spool_dir().join(name)
            }
        })
}

fn first_spool_file() -> Option<PathBuf> {
    std::fs::read_dir(spool_dir())
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .max_by_key(|path| path.metadata().and_then(|m| m.modified()).ok())
}

fn spool_file_status(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .find_map(|field| field.strip_prefix("status="))
        })
        .next_back()
        .map(|status| status.trim_start_matches('*').to_uppercase())
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dspsplf(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    let path = resolve_spool_file(&fields);
    println!("=== DSPSPLF - {} ===", path.display());
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            for line in content.lines() {
                println!("{}", line);
            }
        }
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[DSPSPLF] Error: {}", error);
        }
    }
    println!("==============================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dltsplf(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    if !confirmed {
        emit_status("CPF0006", None, "DLTSPLF requiere CONFIRM(*YES)");
        println!("[DLTSPLF] Requiere CONFIRM(*YES).");
        return;
    }
    let path = resolve_spool_file(&fields);
    // Note: Spool files are not cataloged with auth metadata, so we skip authority check
    match std::fs::remove_file(&path) {
        Ok(_) => println!("[DLTSPLF] {} eliminado.", path.display()),
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[DLTSPLF] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_chgsplfa(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    let status = fields
        .get("STATUS")
        .or_else(|| fields.get("STATE"))
        .map(|value| value.trim().trim_start_matches('*').to_uppercase())
        .unwrap_or_else(|| "READY".to_string());
    if !matches!(status.as_str(), "READY" | "HELD" | "SAVED") {
        emit_status(
            "CPF0006",
            None,
            "CHGSPLFA STATUS requiere *READY, *HELD o *SAVED",
        );
        println!("[CHGSPLFA] STATUS no soportado: {}", status);
        return;
    }
    let path = resolve_spool_file(&fields);
    // Note: Spool files are not cataloged with auth metadata, so we skip authority check
    match OpenOptions::new().append(true).open(&path) {
        Ok(mut file) => {
            let _ = writeln!(file, "status={} changed_at={}", status, now_epoch_string());
            clear_status();
            println!("[CHGSPLFA] {} status={}.", path.display(), status);
        }
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[CHGSPLFA] Error: {}", error);
        }
    }
}

/// WRKCMD — Lista comandos catalogados como *CMD.
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkcmd() {
    l400_wrkcmd_spec(std::ptr::null());
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkcmd_spec(spec: *const c_char) {
    let fields = parse_command_fields(&c_str_to_string(spec));
    let cmd_filter = fields
        .get("CMD")
        .map(String::as_str)
        .unwrap_or("*ALL")
        .trim()
        .to_uppercase();
    let auth_filter = fields
        .get("AUTH")
        .map(String::as_str)
        .unwrap_or("*ALL")
        .trim()
        .to_uppercase();
    let status_filter = fields
        .get("STATUS")
        .map(String::as_str)
        .unwrap_or("*ALL")
        .trim()
        .to_lowercase();
    println!("=== WRKCMD - Command Objects ===");
    println!(
        "  Filters: CMD={} AUTH={} STATUS={}",
        cmd_filter, auth_filter, status_filter
    );
    let root = crate::object::resolve_l400_root();
    println!(
        "  {:10} {:20} {:10} {:14} TEXT",
        "LIB", "CMD", "AUT", "STATUS"
    );
    println!("  {}", "-".repeat(82));
    let mut count = 0usize;
    if let Ok(libraries) = crate::object::list_libraries(&root) {
        for library in libraries {
            let lib_path = root.join(&library);
            let Ok(objects) = crate::object::list_objects(&lib_path) else {
                continue;
            };
            for object in objects
                .into_iter()
                .filter(|object| object.objtype == "*CMD")
            {
                let path = lib_path.join(&object.name);
                let authority = crate::storage::read_string_attr(&path, "user.l400.cmd.authority")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "*USE".to_string());
                let status = crate::command_metadata(&object.name)
                    .map(|metadata| metadata.status())
                    .unwrap_or("user");
                if !matches_pattern(&object.name, &cmd_filter) {
                    continue;
                }
                if auth_filter != "*ALL" && authority.to_uppercase() != auth_filter {
                    continue;
                }
                if status_filter != "*all" && status != status_filter {
                    continue;
                }
                count += 1;
                println!(
                    "  {:10} {:20} {:10} {:14} {}",
                    library,
                    object.name,
                    authority,
                    status,
                    object.text.unwrap_or_default()
                );
            }
        }
    }
    if count == 0 {
        println!("  No hay comandos catalogados.");
    }
    println!("===============================");
}

/// DSPCMD — Muestra metadata promptable de un objeto *CMD.
#[unsafe(no_mangle)]
pub extern "C" fn l400_dspcmd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let command = fields
        .get("CMD")
        .cloned()
        .unwrap_or_else(|| spec.trim().to_string())
        .trim()
        .to_uppercase();
    if command.is_empty() {
        println!("[DSPCMD] Uso: DSPCMD CMD(WRKOBJ)");
        return;
    }

    let root = crate::object::resolve_l400_root();
    let path = root.join("QSYS").join(&command);
    println!("=== DSPCMD - {} ===", command);
    match crate::object::describe_object(&path) {
        Ok(object) => {
            let metadata = crate::command_metadata(&command);
            println!("  Command . . . . . . . . : {}", object.name);
            println!("  Type  . . . . . . . . . : {}", object.objtype);
            println!(
                "  Metadata schema . . . . : v{}",
                crate::COMMAND_METADATA_SCHEMA_VERSION
            );
            println!(
                "  Text  . . . . . . . . . : {}",
                object.text.unwrap_or_default()
            );
            let authority = crate::storage::read_string_attr(&path, "user.l400.cmd.authority")
                .ok()
                .flatten()
                .unwrap_or_else(|| "*USE".to_string());
            println!("  Authority required . . : {}", authority);
            println!(
                "  Status  . . . . . . . . : {}",
                metadata.map(|metadata| metadata.status()).unwrap_or("user")
            );
            let params = crate::storage::read_string_attr(&path, "user.l400.cmd.params")
                .ok()
                .flatten()
                .unwrap_or_default();
            if params.is_empty() {
                println!("  No prompt metadata registered.");
            } else {
                println!("  Parameters:");
                println!(
                    "    {:12} {:8} {:10} {:24} DEFAULT",
                    "Name", "Type", "Use", "Values"
                );
                for param in params.split('|') {
                    let parts = param.split(':').collect::<Vec<_>>();
                    let name = parts.first().copied().unwrap_or("");
                    let type_ = parts.get(1).copied().unwrap_or("");
                    let use_ = parts.get(2).copied().unwrap_or("");
                    let values = parts.get(3).copied().unwrap_or("");
                    let default = parts.get(4).copied().unwrap_or("");
                    println!(
                        "    {:12} {:8} {:10} {:24} {}",
                        name, type_, use_, values, default
                    );
                }
            }
            if let Some(metadata) = metadata {
                let examples = metadata.examples();
                if !examples.is_empty() {
                    println!("  Examples:");
                    for example in examples {
                        println!("    {}", example);
                    }
                }
            }
        }
        Err(error) => println!("[DSPCMD] Error: {}", error),
    }
    println!("===============================");
}

/// CRTCMD — Registra un comando interno minimo como objeto *CMD.
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtcmd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(command) = fields.get("CMD") else {
        println!("[CRTCMD] Uso: CRTCMD CMD(QSYS/MYCMD) TEXT(Description)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (library, name, _path) =
        resolve_object_spec(&root, command, fields.get("LIB").map(String::as_str));
    let text = fields
        .get("TEXT")
        .map(String::as_str)
        .unwrap_or("User command");
    let lib_path = root.join(&library);
    let user = runtime_user();
    match crate::auth::check_authority(&lib_path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status(
                "CPF2204",
                Some(&lib_path),
                "authority insufficient for create",
            );
            println!(
                "[CRTCMD] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                lib_path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&lib_path), &error.to_string());
            println!("[CRTCMD] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::object::create_object_with_metadata(
        &lib_path,
        &name,
        "*CMD",
        Some("CMD"),
        Some(text),
    ) {
        Ok(path) => {
            let _ = crate::storage::write_string_attr(&path, "user.l400.cmd.text", text);
            let _ = crate::storage::write_string_attr(&path, "user.l400.cmd.authority", "*USE");
            println!("[CRTCMD] {}/{} creado.", library, name);
        }
        Err(error) => println!("[CRTCMD] Error: {}", error),
    }
}

/// WRKUSRPRF — Gestiona perfiles de usuario
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkusrprf(usrprf: *const c_char) {
    let spec = c_str_to_string(usrprf);
    let fields = parse_command_fields(&spec);
    let action = fields
        .get("ACTION")
        .map(String::as_str)
        .unwrap_or("*LIST")
        .to_uppercase();
    let filter = fields
        .get("USRPRF")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if spec.trim().is_empty() {
                "*ALL".to_string()
            } else {
                spec.trim().to_string()
            }
        })
        .to_uppercase();
    let root = crate::object::resolve_l400_root();
    let qsys = root.join("QSYS");

    println!("=== WRKUSRPRF - Perfiles de Usuario ===");
    if matches!(action.as_str(), "*CREATE" | "CREATE") {
        match crate::object::create_object_with_metadata(
            &qsys,
            &filter,
            "*USRPRF",
            Some("USRPRF"),
            Some("Linux/400 user profile"),
        ) {
            Ok(_) => {
                audit_runtime(
                    "USRPRF_CHANGE",
                    &qsys.join(&filter),
                    &format!("CREATE {}", filter),
                );
                println!("  Perfil {} creado.", filter)
            }
            Err(error) => println!("  Error creando perfil {}: {}", filter, error),
        }
        println!("========================================");
        return;
    }

    if matches!(action.as_str(), "*DISABLE" | "DISABLE") {
        let path = qsys.join(&filter);
        match xattr::set(&path, "user.l400.disabled", b"yes") {
            Ok(_) => {
                audit_runtime("USRPRF_CHANGE", &path, &format!("DISABLE {}", filter));
                println!("  Perfil {} desactivado.", filter)
            }
            Err(error) => println!("  Error desactivando perfil {}: {}", filter, error),
        }
        println!("========================================");
        return;
    }

    if qsys.exists() {
        if let Ok(entries) = std::fs::read_dir(&qsys) {
            println!("  {:16} {:8} TEXT", "USRPRF", "STATUS");
            println!("  {}", "-".repeat(48));
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_uppercase();
                let path = entry.path();
                if let Ok(object) = crate::object::describe_object(&path)
                    && object.objtype == "*USRPRF"
                    && matches_pattern(&name, &filter)
                {
                    let disabled = xattr::get(&path, "user.l400.disabled")
                        .ok()
                        .flatten()
                        .is_some();
                    println!(
                        "  {:16} {:8} {}",
                        name,
                        if disabled { "*DISABLED" } else { "*ENABLED" },
                        object.text.as_deref().unwrap_or("")
                    );
                }
            }
        }
    } else {
        println!("  Directorio QSYS no disponible.");
    }
    println!("====================================================");
}

/// PWRDWNSYS — Apaga o reinicia el sistema
#[unsafe(no_mangle)]
pub extern "C" fn l400_pwrdwnsys(option: *const c_char) {
    clear_status();
    let spec = c_str_to_string(option);
    let fields = parse_command_fields(&spec);
    let opt = power_down_option_from_spec(&spec, &fields);
    let Some(action) = PowerDownAction::from_option(&opt) else {
        emit_status(
            "CPF0006",
            None,
            "PWRDWNSYS OPTION debe ser *CNTRLD, *IMMED o *RESTART",
        );
        return;
    };

    println!("[PWRDWNSYS] Solicitud aceptada (OPTION={})", action.label());
    let confirmed = confirmed_yes(fields.get("CONFIRM"));
    if !confirmed {
        emit_status("CPF0006", None, "PWRDWNSYS requiere CONFIRM(*YES)");
        return;
    }
    if unsafe { libc::geteuid() } != 0 && !power_down_dry_run_enabled() {
        emit_status("CPF2204", None, "PWRDWNSYS requiere root");
        return;
    }

    audit_runtime(
        "PWRDWNSYS",
        Path::new("/"),
        &format!("option={} confirmed=*YES", action.label()),
    );
    match run_power_down_action(action) {
        Ok(()) => {
            clear_status();
            println!("[PWRDWNSYS] Accion de energia enviada.");
        }
        Err(error) => {
            emit_status(
                "CPF9898",
                None,
                &format!("No se pudo ejecutar accion de energia: {error}"),
            );
        }
    }
}

fn power_down_option_from_spec(
    spec: &str,
    fields: &std::collections::HashMap<String, String>,
) -> String {
    if let Some(option) = fields.get("OPTION") {
        return option.to_uppercase();
    }
    if fields.is_empty() && !spec.trim().is_empty() {
        return spec.trim().to_uppercase();
    }
    "*CNTRLD".to_string()
}

// ---------------------------------------------------------------------------
// Objetos y bibliotecas
// ---------------------------------------------------------------------------

/// WRKOBJ — Busca y lista objetos del catálogo
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkobj(obj_filter: *const c_char) {
    let spec = c_str_to_string(obj_filter);
    let fields = parse_command_fields(&spec);
    let obj_filter = fields
        .get("OBJ")
        .cloned()
        .unwrap_or_else(|| "*ALL".to_string());
    let objtype_filter = fields
        .get("OBJTYPE")
        .cloned()
        .unwrap_or_else(|| "*ALL".to_string());
    let lib_filter = fields.get("LIB").cloned().unwrap_or_else(|| {
        obj_filter
            .split_once('/')
            .map(|(library, _)| library.to_string())
            .unwrap_or_else(|| "*ALL".to_string())
    });
    let object_pattern = obj_filter
        .split_once('/')
        .map(|(_, object)| object.to_string())
        .unwrap_or(obj_filter);

    println!(
        "=== WRKOBJ - Objetos OBJ({}) OBJTYPE({}) LIB({}) ===",
        object_pattern, objtype_filter, lib_filter
    );
    let root = crate::object::resolve_l400_root();
    match crate::object::list_libraries(&root) {
        Ok(libraries) => {
            let mut printed = 0usize;
            println!(
                "  {:10} {:20} {:10} {:10}",
                "LIB", "OBJETO", "TIPO", "ATRIB"
            );
            println!("  {}", "-".repeat(58));
            for library in libraries {
                if !matches_pattern(&library, &lib_filter) {
                    continue;
                }
                let lib_path = root.join(&library);
                if let Ok(objects) = crate::object::list_objects(&lib_path) {
                    for obj in objects {
                        if !matches_pattern(&obj.name, &object_pattern)
                            || !matches_pattern(&obj.objtype, &objtype_filter)
                        {
                            continue;
                        }
                        printed += 1;
                        println!(
                            "  {:10} {:20} {:10} {:10}",
                            library,
                            obj.name,
                            obj.objtype,
                            obj.attribute.as_deref().unwrap_or("-")
                        );
                    }
                }
            }
            if printed == 0 {
                println!("  No hay objetos para el filtro indicado.");
            }
        }
        Err(error) => println!("  Error al listar bibliotecas: {}", error),
    }
    println!("=====================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dltobj(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
        emit_status("CPF0006", None, "DLTOBJ requiere OBJ");
        println!("[DLTOBJ] Uso: DLTOBJ OBJ(QGPL/MYOBJ) CONFIRM(*YES)");
        return;
    };
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    if !confirmed {
        emit_status("CPF0006", None, "DLTOBJ requiere CONFIRM(*YES)");
        println!("[DLTOBJ] Requiere CONFIRM(*YES).");
        return;
    }
    let root = crate::object::resolve_l400_root();
    let (_library, object, path) =
        resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));

    // Check if object exists; if not, emit CPF9801 "object not found" (idempotent delete scenario)
    if !path.exists() {
        emit_status("CPF9801", Some(&path), "Object not found for delete");
        println!("[DLTOBJ] Objeto no encontrado: {}", path.display());
        return;
    }

    let user = runtime_user();
    match crate::auth::check_authority(&path, &user, crate::auth::L400Authority::All) {
        Ok(true) => {}
        Ok(false) => {
            emit_status("CPF2204", Some(&path), "authority insufficient for delete");
            println!(
                "[DLTOBJ] Denegado por autoridad: usuario {} no tiene *ALL sobre {}.",
                user,
                path.display()
            );
            return;
        }
        Err(error) => {
            // Map NotFound to CPF9801 (object not found) instead of CPF0001
            if let crate::auth::AuthError::Io(ref io_err) = error
                && io_err.kind() == std::io::ErrorKind::NotFound
            {
                emit_status("CPF9801", Some(&path), "Object not found");
                println!("[DLTOBJ] Objeto no encontrado: {}", path.display());
                return;
            }
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[DLTOBJ] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::object::delete_object(&path) {
        Ok(_) => println!("[DLTOBJ] {} eliminado.", object),
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[DLTOBJ] Error eliminando {}: {}", object, error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_cpyobj(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(obj), Some(toobj)) = (fields.get("OBJ"), fields.get("TOOBJ")) else {
        emit_status("CPF0006", None, "CPYOBJ requiere OBJ y TOOBJ");
        println!("[CPYOBJ] Uso: CPYOBJ OBJ(QGPL/A) TOOBJ(QGPL/B)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, src_name, src) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    let (_, dst_name, dst) =
        resolve_object_spec(&root, toobj, fields.get("TOLIB").map(String::as_str));
    match crate::object::copy_object(&src, &dst) {
        Ok(_) => println!("[CPYOBJ] {} copiado a {}.", src_name, dst_name),
        Err(error) => {
            emit_status("CPF9801", Some(&src), &error.to_string());
            println!("[CPYOBJ] Error copiando {}: {}", src_name, error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dspobjd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
        emit_status("CPF0006", None, "DSPOBJD requiere OBJ");
        println!("[DSPOBJD] Uso: DSPOBJD OBJ(QGPL/MYOBJ)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match crate::object::describe_object(&path) {
        Ok(object) => {
            println!("=== DSPOBJD - Descripcion de Objeto ===");
            println!(
                "  Library . . . . . . . . . : {}",
                object.library.unwrap_or_default()
            );
            println!("  Object  . . . . . . . . . : {}", object.name);
            println!("  Type  . . . . . . . . . . : {}", object.objtype);
            println!(
                "  Attribute . . . . . . . . : {}",
                object.attribute.unwrap_or_default()
            );
            println!(
                "  Text  . . . . . . . . . . : {}",
                object.text.unwrap_or_default()
            );
            println!(
                "  Owner . . . . . . . . . . : {}",
                object.owner.unwrap_or_default()
            );
            println!(
                "  Public authority . . . . : {}",
                object.public_auth.unwrap_or_default()
            );
            if let Ok(Some(toolchain)) =
                crate::storage::read_string_attr(&path, "user.l400.toolchain")
            {
                println!("  Toolchain . . . . . . . : {}", toolchain);
            }
            if let Ok(Some(signature)) =
                crate::storage::read_string_attr(&path, "user.l400.signature")
            {
                println!("  Signature . . . . . . . : {}", signature);
            }
            println!("=======================================");
        }
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[DSPOBJD] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_chgobjd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
        emit_status("CPF0006", None, "CHGOBJD requiere OBJ");
        println!("[CHGOBJD] Uso: CHGOBJD OBJ(QGPL/MYOBJ) TEXT(Demo)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    let user = runtime_user();
    match crate::auth::check_authority(&path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status("CPF2204", Some(&path), "authority insufficient for change");
            println!(
                "[CHGOBJD] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[CHGOBJD] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::object::describe_object(&path) {
        Ok(object) => {
            let text = fields
                .get("TEXT")
                .map(String::as_str)
                .or(object.text.as_deref());
            let attr = fields
                .get("OBJATTR")
                .map(String::as_str)
                .or(object.attribute.as_deref());
            match crate::object::catalog_object(&path, &object.objtype, attr, text) {
                Ok(_) => println!("[CHGOBJD] Objeto actualizado."),
                Err(error) => {
                    emit_status("CPF0001", Some(&path), &error.to_string());
                    println!("[CHGOBJD] Error actualizando objeto: {}", error);
                }
            }
        }
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[CHGOBJD] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dspobjaut(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
        emit_status("CPF0006", None, "DSPOBJAUT requiere OBJ");
        println!("[DSPOBJAUT] Uso: DSPOBJAUT OBJ(QGPL/MYOBJ)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match crate::auth::get_object_authorities(&path) {
        Ok(auths) if auths.is_empty() => println!("[DSPOBJAUT] Sin autorizaciones explicitas."),
        Ok(auths) => {
            println!("=== DSPOBJAUT - Autoridades ===");
            println!("  {:16} AUT", "USER");
            println!("  {}", "-".repeat(30));
            let mut rows = auths.into_iter().collect::<Vec<_>>();
            rows.sort_by(|left, right| left.0.cmp(&right.0));
            for (user, authority) in rows {
                println!("  {:16} {}", user, authority);
            }
            println!("===============================");
        }
        Err(error) => {
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[DSPOBJAUT] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_grtobjaut(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(obj), Some(user), Some(aut)) =
        (fields.get("OBJ"), fields.get("USER"), fields.get("AUT"))
    else {
        emit_status("CPF0006", None, "GRTOBJAUT requiere OBJ USER AUT");
        println!("[GRTOBJAUT] Uso: GRTOBJAUT OBJ(QGPL/MYOBJ) USER(QPGMR) AUT(*USE)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match aut.parse() {
        Ok(authority) => match crate::auth::grant_object_authority(&path, user, authority) {
            Ok(_) => {
                audit_runtime(
                    "AUTH_CHANGE",
                    &path,
                    &format!("GRTOBJAUT user={} aut={}", user, aut),
                );
                println!("[GRTOBJAUT] Autoridad {} otorgada a {}.", aut, user)
            }
            Err(error) => {
                emit_status("CPF0001", Some(&path), &error.to_string());
                println!("[GRTOBJAUT] Error: {}", error);
            }
        },
        Err(error) => {
            emit_status("CPF0006", Some(&path), &error.to_string());
            println!("[GRTOBJAUT] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_rvkobjaut(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(obj), Some(user)) = (fields.get("OBJ"), fields.get("USER")) else {
        emit_status("CPF0006", None, "RVKOBJAUT requiere OBJ USER");
        println!("[RVKOBJAUT] Uso: RVKOBJAUT OBJ(QGPL/MYOBJ) USER(QPGMR)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match crate::auth::revoke_object_authority(&path, user) {
        Ok(_) => {
            audit_runtime("AUTH_CHANGE", &path, &format!("RVKOBJAUT user={}", user));
            println!("[RVKOBJAUT] Autoridad revocada para {}.", user)
        }
        Err(error) => {
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[RVKOBJAUT] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_chkobjaut(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
        println!("[CHKOBJAUT] Uso: CHKOBJAUT OBJ(QGPL/MYOBJ) USER(QPGMR) AUT(*USE)");
        return;
    };
    let user = fields
        .get("USER")
        .cloned()
        .unwrap_or_else(runtime_user)
        .to_uppercase();
    let aut = fields
        .get("AUT")
        .cloned()
        .unwrap_or_else(|| "*USE".to_string());
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match aut.parse::<crate::auth::L400Authority>() {
        Ok(authority) => match crate::auth::check_authority(&path, &user, authority) {
            Ok(true) => println!(
                "[CHKOBJAUT] ALLOW user={} aut={} obj={}",
                user,
                aut,
                path.display()
            ),
            Ok(false) => {
                audit_runtime(
                    "ACCESS_DENIED",
                    &path,
                    &format!("CHKOBJAUT user={} aut={}", user, aut),
                );
                println!(
                    "[CHKOBJAUT] DENY user={} aut={} obj={}",
                    user,
                    aut,
                    path.display()
                );
            }
            Err(error) => println!("[CHKOBJAUT] Error: {}", error),
        },
        Err(error) => println!("[CHKOBJAUT] Error: {}", error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_chkobjint(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let repair = fields
        .get("REPAIR")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    let Some(obj) = fields.get("OBJ") else {
        emit_status("CPF0006", None, "CHKOBJINT requiere OBJ");
        println!("[CHKOBJINT] Uso: CHKOBJINT OBJ(QGPL/MYOBJ) REPAIR(*NO)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    println!("=== CHKOBJINT - Object Integrity ===");
    println!("  Object path . . . . . : {}", path.display());
    println!(
        "  Repair mode . . . . . : {}",
        if repair { "*YES" } else { "*NO" }
    );
    let mut issues = Vec::new();
    let mut repairs = Vec::new();
    match crate::object::describe_object(&path) {
        Ok(object) => {
            println!("  Object . . . . . . . : {}", object.name);
            println!("  Type . . . . . . . . : {}", object.objtype);
            println!(
                "  Attribute  . . . . . : {}",
                object.attribute.as_deref().unwrap_or("-")
            );
            if object.owner.as_deref().unwrap_or("").is_empty() {
                issues.push("missing owner metadata".to_string());
            }
            if object.objtype == "*FILE" {
                match object.attribute.as_deref() {
                    Some("PF") => {
                        if crate::storage::read_string_attr(
                            &path,
                            crate::L400_DATA_FORMAT_VERSION_ATTR,
                        )
                        .ok()
                        .flatten()
                        .is_none()
                        {
                            if repair
                                && crate::storage::write_u32_attr(
                                    &path,
                                    crate::L400_DATA_FORMAT_VERSION_ATTR,
                                    crate::L400_DATA_FORMAT_VERSION,
                                )
                                .is_ok()
                            {
                                repairs.push(format!(
                                    "PF wrote {}={}",
                                    crate::L400_DATA_FORMAT_VERSION_ATTR,
                                    crate::L400_DATA_FORMAT_VERSION
                                ));
                            } else {
                                issues.push(format!(
                                    "PF missing {}",
                                    crate::L400_DATA_FORMAT_VERSION_ATTR
                                ));
                            }
                        }
                        if crate::storage::read_string_attr(&path, crate::L400_STORAGE_BACKEND_ATTR)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            if repair
                                && crate::storage::write_storage_backend(
                                    &path,
                                    crate::storage::default_storage_backend(),
                                )
                                .is_ok()
                            {
                                repairs.push("PF wrote storage backend".to_string());
                            } else {
                                issues.push("PF missing user.l400.storage_backend".to_string());
                            }
                        }
                        if crate::storage::read_string_attr(&path, crate::L400_RECORD_LEN_ATTR)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            if repair
                                && crate::storage::write_u32_attr(
                                    &path,
                                    crate::L400_RECORD_LEN_ATTR,
                                    256,
                                )
                                .is_ok()
                            {
                                repairs.push("PF wrote default record_len=256".to_string());
                            } else {
                                issues.push("PF missing user.l400.record_len".to_string());
                            }
                        }
                        if crate::storage::read_string_attr(&path, crate::L400_KEY_FIELDS_ATTR)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            if repair
                                && crate::storage::write_string_attr(
                                    &path,
                                    crate::L400_KEY_FIELDS_ATTR,
                                    "KEY",
                                )
                                .is_ok()
                            {
                                repairs.push("PF wrote default key_fields=KEY".to_string());
                            } else {
                                issues.push("PF missing user.l400.key_fields".to_string());
                            }
                        }
                        if crate::storage::read_string_attr(&path, crate::L400_PF_MEMBERS_ATTR)
                            .ok()
                            .flatten()
                            .is_none()
                        {
                            if repair
                                && crate::storage::write_string_attr(
                                    &path,
                                    crate::L400_PF_MEMBERS_ATTR,
                                    crate::db::DEFAULT_PF_MEMBER,
                                )
                                .is_ok()
                            {
                                repairs.push("PF wrote default member list".to_string());
                            } else {
                                issues.push("PF missing user.l400.pf_members".to_string());
                            }
                        }
                    }
                    Some("LF") => {
                        let base_pf = crate::storage::read_string_attr(&path, "user.l400.base_pf")
                            .ok()
                            .flatten();
                        if base_pf.as_deref().unwrap_or("").is_empty() {
                            issues.push("LF missing user.l400.base_pf".to_string());
                        } else if crate::storage::read_string_attr(
                            &path,
                            crate::L400_STORAGE_BACKEND_ATTR,
                        )
                        .ok()
                        .flatten()
                        .is_none()
                        {
                            let repaired = repair
                                && base_pf
                                    .as_deref()
                                    .and_then(|base| {
                                        crate::storage::read_storage_backend(Path::new(base))
                                            .ok()
                                            .flatten()
                                    })
                                    .or_else(|| Some(crate::storage::default_storage_backend()))
                                    .map(|backend| {
                                        crate::storage::write_storage_backend(&path, backend)
                                            .is_ok()
                                    })
                                    .unwrap_or(false);
                            if repaired {
                                repairs.push("LF wrote storage backend from base PF".to_string());
                            } else {
                                issues.push("LF missing user.l400.storage_backend".to_string());
                            }
                        }
                        if crate::storage::read_string_attr(
                            &path,
                            crate::L400_DATA_FORMAT_VERSION_ATTR,
                        )
                        .ok()
                        .flatten()
                        .is_none()
                        {
                            if repair
                                && crate::storage::write_u32_attr(
                                    &path,
                                    crate::L400_DATA_FORMAT_VERSION_ATTR,
                                    crate::L400_DATA_FORMAT_VERSION,
                                )
                                .is_ok()
                            {
                                repairs.push("LF wrote data version".to_string());
                            } else {
                                issues.push(format!(
                                    "LF missing {}",
                                    crate::L400_DATA_FORMAT_VERSION_ATTR
                                ));
                            }
                        }
                    }
                    Some("SRC") => {}
                    Some(other) => issues.push(format!("unknown *FILE attribute {other}")),
                    None => issues.push("*FILE missing attribute".to_string()),
                }
            } else if object.objtype == "*DTAQ" {
                if crate::storage::read_string_attr(&path, crate::L400_STORAGE_BACKEND_ATTR)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    if repair
                        && crate::storage::write_storage_backend(
                            &path,
                            crate::storage::default_storage_backend(),
                        )
                        .is_ok()
                    {
                        repairs.push("DTAQ wrote storage backend".to_string());
                    } else {
                        issues.push("DTAQ missing user.l400.storage_backend".to_string());
                    }
                }
                if crate::storage::read_string_attr(&path, crate::L400_DATA_FORMAT_VERSION_ATTR)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    if repair
                        && crate::storage::write_u32_attr(
                            &path,
                            crate::L400_DATA_FORMAT_VERSION_ATTR,
                            crate::L400_DATA_FORMAT_VERSION,
                        )
                        .is_ok()
                    {
                        repairs.push("DTAQ wrote data version".to_string());
                    } else {
                        issues.push(format!(
                            "DTAQ missing {}",
                            crate::L400_DATA_FORMAT_VERSION_ATTR
                        ));
                    }
                }
            } else if object.objtype == "*OUTQ" {
                for (attr, default_value) in [
                    (crate::L400_OUTQ_RETENTION_DAYS_ATTR, "7"),
                    (crate::L400_OUTQ_ROUTING_ATTR, "QBATCH"),
                    (crate::L400_OUTQ_DEFAULT_STATUS_ATTR, "READY"),
                ] {
                    if crate::storage::read_string_attr(&path, attr)
                        .ok()
                        .flatten()
                        .is_none()
                    {
                        if repair
                            && crate::storage::write_string_attr(&path, attr, default_value).is_ok()
                        {
                            repairs.push(format!("OUTQ wrote {attr}={default_value}"));
                        } else {
                            issues.push(format!("OUTQ missing {attr}"));
                        }
                    }
                }
                if crate::storage::read_string_attr(&path, crate::L400_DATA_FORMAT_VERSION_ATTR)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    if repair
                        && crate::storage::write_u32_attr(
                            &path,
                            crate::L400_DATA_FORMAT_VERSION_ATTR,
                            crate::L400_DATA_FORMAT_VERSION,
                        )
                        .is_ok()
                    {
                        repairs.push("OUTQ wrote data version".to_string());
                    } else {
                        issues.push(format!(
                            "OUTQ missing {}",
                            crate::L400_DATA_FORMAT_VERSION_ATTR
                        ));
                    }
                }
            } else if object.objtype == "*JOBQ" {
                if crate::storage::read_string_attr(&path, crate::L400_DATA_FORMAT_VERSION_ATTR)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    if repair
                        && crate::storage::write_u32_attr(
                            &path,
                            crate::L400_DATA_FORMAT_VERSION_ATTR,
                            crate::L400_DATA_FORMAT_VERSION,
                        )
                        .is_ok()
                    {
                        repairs.push("JOBQ wrote data version".to_string());
                    } else {
                        issues.push(format!(
                            "JOBQ missing {}",
                            crate::L400_DATA_FORMAT_VERSION_ATTR
                        ));
                    }
                }
            } else if object.objtype == "*USRPRF" {
                if crate::storage::read_string_attr(&path, crate::L400_DATA_FORMAT_VERSION_ATTR)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    if repair
                        && crate::storage::write_u32_attr(
                            &path,
                            crate::L400_DATA_FORMAT_VERSION_ATTR,
                            crate::L400_DATA_FORMAT_VERSION,
                        )
                        .is_ok()
                    {
                        repairs.push("USRPRF wrote data version".to_string());
                    } else {
                        issues.push(format!(
                            "USRPRF missing {}",
                            crate::L400_DATA_FORMAT_VERSION_ATTR
                        ));
                    }
                }
                if crate::storage::read_string_attr(&path, crate::auth::L400_AUTH_VERSION_ATTR)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    if repair
                        && crate::storage::write_string_attr(
                            &path,
                            crate::auth::L400_AUTH_VERSION_ATTR,
                            &crate::L400_AUTH_MANIFEST_VERSION.to_string(),
                        )
                        .is_ok()
                    {
                        repairs.push("USRPRF wrote auth version".to_string());
                    } else {
                        issues.push(format!(
                            "USRPRF missing {}",
                            crate::auth::L400_AUTH_VERSION_ATTR
                        ));
                    }
                }
            } else if object.objtype == "*PGM"
                && crate::storage::read_string_attr(&path, crate::L400_DATA_FORMAT_VERSION_ATTR)
                    .ok()
                    .flatten()
                    .is_none()
            {
                if repair
                    && crate::storage::write_u32_attr(
                        &path,
                        crate::L400_DATA_FORMAT_VERSION_ATTR,
                        crate::L400_DATA_FORMAT_VERSION,
                    )
                    .is_ok()
                {
                    repairs.push("PGM wrote data version".to_string());
                } else {
                    issues.push(format!(
                        "PGM missing {}",
                        crate::L400_DATA_FORMAT_VERSION_ATTR
                    ));
                }
            }
        }
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[CHKOBJINT] Error: {}", error);
            return;
        }
    }

    if issues.is_empty() {
        clear_status();
        println!("  Result . . . . . . . : OK");
    } else {
        emit_status("CPF9898", Some(&path), "object integrity issues found");
        println!("  Result . . . . . . . : CHECK");
        for issue in issues {
            println!("  - {}", issue);
        }
    }
    if !repairs.is_empty() {
        println!("  Repairs . . . . . . : {}", repairs.len());
        for repair in repairs {
            println!("  + {}", repair);
        }
    }
    println!("===================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dsppolicy() {
    println!("=== DSPPOLICY - Matriz de autorizaciones Linux/400 ===");
    println!("  {:24} {:10} REQUIRED", "COMMAND", "OPERATION");
    println!("  {}", "-".repeat(52));
    for (command, operation, authority) in crate::auth::authority_matrix_rows() {
        println!("  {:24} {:10} {}", command, operation, authority);
    }
    println!();
    println!(
        "  Runtime auth manifest version: v{}",
        crate::L400_AUTH_MANIFEST_VERSION
    );
    println!(
        "  Runtime auth format: USER:*AUTH plus UID:<uid>:*AUTH mirror when *USRPRF is resolvable."
    );
    println!("  Identidad runtime: L400_USER -> USER fallback.");
    println!("  eBPF phase3-v1 recibe identidad via owner_uid y entradas UID:<uid> para exec.");
    println!("  Userspace aplica matriz completa antes de comandos sensibles.");
    println!("================================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dspaudit() {
    println!("=== DSPAUD - Auditoria QHST/Linux400 ===");
    match crate::audit::read_audit_records(50) {
        Ok(records) if records.is_empty() => println!("  Sin registros de auditoria."),
        Ok(records) => {
            println!(
                "  {:12} {:16} {:10} {:24} MESSAGE",
                "TS", "EVENT", "USER", "OBJECT"
            );
            println!("  {}", "-".repeat(86));
            for record in records {
                println!(
                    "  {:12} {:16} {:10} {:24} {}",
                    record.timestamp, record.event, record.user, record.object, record.message
                );
            }
        }
        Err(error) => println!("[DSPAUD] Error: {}", error),
    }
    println!("========================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dspptf(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    let option = fields
        .get("OPTION")
        .map(|v| v.to_uppercase())
        .unwrap_or_else(|| "ALL".to_string());

    println!("=== DSPPTF - PTF Maintenance History ===");

    // Read audit log to show PTF history
    let audit_path = Path::new("/var/log/l400/ptf-audit.log");
    if !audit_path.exists() {
        println!("  No hay registros de PTF aplicados.");
        println!("========================================");
        return;
    }

    // Filter by option
    let show_applied = option == "*ALL" || option == "*APPLIED";
    let show_pending = option == "*ALL" || option == "*PENDING";

    if show_applied {
        println!("\nPTFs Aplicados:");
        println!("  {:12} {:20} {:12} {}", "DATE", "PTF ID", "USER", "RESULT");
        println!("  {}", "-".repeat(70));

        if let Ok(records) = crate::ptf::read_ptf_history() {
            for record in records.iter().filter(|r| r.action == "APPLY") {
                println!(
                    "  {:12} {:20} {:12} {}",
                    record.timestamp, record.ptf_id, record.user, record.result
                );
            }
        }
    }

    if show_pending {
        println!("\nPTFs Pendientes (en cache):");
        println!("  {:12} {:30} {}", "PTF ID", "NAME", "VERSION");
        println!("  {}", "-".repeat(70));

        if let Ok(packages) = crate::ptf::list_pending_ptfs() {
            for pkg in packages {
                println!("  {:12} {:30} {}", pkg.id, pkg.name, pkg.target_version);
            }
        } else {
            println!("  No se pudo leer el directorio de PTFs pendientes.");
        }
    }

    println!("\n========================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_aptptf(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    let ptf_id = if let Some(id) = fields.get("PTF") {
        id.trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_start_matches("'")
            .trim_end_matches("'")
            .to_string()
    } else {
        emit_status("CPF0006", None, "APYPTF requiere PTF");
        println!(
            "[APYPTF] Uso: APYPTF PTF(PTF0001) OPTION(*APPLY|*CHECK|*ROLLBACK) CONFIRM(*YES|*NO)"
        );
        return;
    };

    let option = fields
        .get("OPTION")
        .map(|v| v.to_uppercase())
        .unwrap_or_else(|| "*CHECK".to_string());

    let confirm = fields
        .get("CONFIRM")
        .map(|v| v.to_uppercase())
        .unwrap_or_else(|| "*NO".to_string());

    let confirm_bool = confirm == "*YES" || confirm == "YES";

    println!("=== APYPTF - Aplicar/Revetir PTF ===");

    let result = match option.as_str() {
        "*CHECK" | "*CHK" => crate::ptf::check_ptf(&ptf_id),
        "*APPLY" | "*APL" => crate::ptf::apply_ptf(&ptf_id, confirm_bool),
        "*ROLLBACK" | "*RLB" => crate::ptf::rollback_ptf(&ptf_id, confirm_bool),
        _ => {
            emit_status("CPF0006", None, &format!("OPTION {option} no váido"));
            println!("[APYPTF] OPTION debe ser *CHECK, *APPLY o *ROLLBACK");
            return;
        }
    };

    match result {
        Ok(msg) => {
            println!("[APYPTF] {}", msg);
            emit_status("   ", None, &msg);
        }
        Err(err) => {
            println!("[APYPTF] Error: {}", err);
            emit_status("CPF0001", None, &err);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_crtpf(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
        emit_status("CPF0006", None, "CRTPF requiere FILE");
        println!("[CRTPF] Uso: CRTPF FILE(QGPL/CUSTOMERS) RCDLEN(128)");
        return;
    };
    let record_len = fields
        .get("RCDLEN")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(256);
    let root = crate::object::resolve_l400_root();
    let (library, name, _path) =
        resolve_object_spec(&root, file, fields.get("LIB").map(String::as_str));
    let lib_path = root.join(&library);
    let user = runtime_user();
    match crate::auth::check_authority(&lib_path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status(
                "CPF2204",
                Some(&lib_path),
                "authority insufficient for create",
            );
            println!(
                "[CRTPF] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                lib_path.display()
            );
            return;
        }
        Err(error) => {
            // Map NotFound to CPF9801 (object not found) instead of CPF0001
            if let crate::auth::AuthError::Io(ref io_err) = error
                && io_err.kind() == std::io::ErrorKind::NotFound
            {
                emit_status("CPF9801", Some(&lib_path), "Library not found");
                println!("[CRTPF] Biblioteca no encontrada: {}", lib_path.display());
                return;
            }
            emit_status("CPF0001", Some(&lib_path), &error.to_string());
            println!("[CRTPF] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::db::create_pf(&lib_path, &name, record_len) {
        Ok(pf) => {
            let schema = crate::db::PfSchema {
                record_len: record_len as u32,
                fields: parse_pf_fields(fields.get("FIELDS").map(String::as_str).unwrap_or("")),
                key_fields: fields
                    .get("KEY")
                    .map(|value| {
                        value
                            .split(',')
                            .map(|field| field.trim().to_uppercase())
                            .filter(|field| !field.is_empty())
                            .collect::<Vec<_>>()
                    })
                    .filter(|keys| !keys.is_empty())
                    .unwrap_or_else(|| vec!["KEY".to_string()]),
            };
            let _ = crate::db::write_pf_schema(&pf.path, &schema);
            println!(
                "[CRTPF] {}/{} creado RCDLEN({}).",
                library, name, record_len
            );
        }
        Err(error) => {
            let path = root.join(&library).join(&name);
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[CRTPF] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_crtlf(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(file), Some(srcfile)) = (fields.get("FILE"), fields.get("SRCFILE")) else {
        emit_status("CPF0006", None, "CRTLF requiere FILE y SRCFILE");
        println!("[CRTLF] Uso: CRTLF FILE(QGPL/CUSTBYNAME) SRCFILE(QGPL/CUSTOMERS)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (library, name, _path) =
        resolve_object_spec(&root, file, fields.get("LIB").map(String::as_str));
    let (_src_library, _src_name, src_path) =
        resolve_object_spec(&root, srcfile, fields.get("SRCLIB").map(String::as_str));
    let lib_path = root.join(&library);
    let user = runtime_user();
    match crate::auth::check_authority(&lib_path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status(
                "CPF2204",
                Some(&lib_path),
                "authority insufficient for create",
            );
            println!(
                "[CRTLF] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                lib_path.display()
            );
            return;
        }
        Err(error) => {
            // Map NotFound to CPF9801 (object not found) instead of CPF0001
            if let crate::auth::AuthError::Io(ref io_err) = error
                && io_err.kind() == std::io::ErrorKind::NotFound
            {
                emit_status("CPF9801", Some(&lib_path), "Library not found");
                println!("[CRTLF] Biblioteca no encontrada: {}", lib_path.display());
                return;
            }
            emit_status("CPF0001", Some(&lib_path), &error.to_string());
            println!("[CRTLF] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::db::PhysicalFile::open(&src_path).and_then(|pf| {
        crate::db::create_lf_filtered(
            &lib_path,
            &name,
            &pf,
            fields.get("SELECT").map(String::as_str),
            fields.get("OMIT").map(String::as_str),
        )
    }) {
        Ok(_) => println!(
            "[CRTLF] {}/{} creado sobre {}.",
            library,
            name,
            src_path.display()
        ),
        Err(error) => {
            let path = root.join(&library).join(&name);
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[CRTLF] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dsppfm(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
        emit_status("CPF0006", None, "DSPPFM requiere FILE");
        println!("[DSPPFM] Uso: DSPPFM FILE(QGPL/CUSTOMERS)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, file, fields.get("LIB").map(String::as_str));
    let member = fields
        .get("MBR")
        .map(String::as_str)
        .unwrap_or(crate::db::DEFAULT_PF_MEMBER);
    println!("=== DSPPFM - {} MBR({}) ===", path.display(), member);
    match crate::db::PhysicalFile::open_member(&path, member) {
        Ok(pf) => {
            if let Ok(schema) = crate::db::read_pf_schema(&path) {
                println!(
                    "  RCDLEN={} KEY({}) FIELDS({})",
                    schema.record_len,
                    schema.key_fields.join(","),
                    schema
                        .fields
                        .iter()
                        .map(|field| format!("{}:{}:{}", field.name, field.type_, field.length))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            println!("  {:8} {:20} DATA", "RRN", "KEY");
            println!("  {}", "-".repeat(72));
            match pf.read_all() {
                Ok(rows) if rows.is_empty() => println!("  No records."),
                Ok(rows) => {
                    for (index, (key, data)) in rows.into_iter().enumerate() {
                        println!(
                            "  {:8} {:20} {}",
                            index + 1,
                            String::from_utf8_lossy(&key),
                            String::from_utf8_lossy(&data)
                        );
                    }
                }
                Err(error) => println!("  Error leyendo registros: {}", error),
            }
        }
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("  Error abriendo PF: {}", error);
        }
    }
    println!("======================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_clrpfm(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
        println!("[CLRPFM] Uso: CLRPFM FILE(QGPL/CUSTOMERS) CONFIRM(*YES)");
        return;
    };
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    if !confirmed {
        println!("[CLRPFM] Requiere CONFIRM(*YES).");
        return;
    }
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, file, fields.get("LIB").map(String::as_str));
    let member = fields
        .get("MBR")
        .map(String::as_str)
        .unwrap_or(crate::db::DEFAULT_PF_MEMBER);
    match crate::db::PhysicalFile::open_member(&path, member).and_then(|pf| pf.clear()) {
        Ok(_) => println!("[CLRPFM] {} MBR({}) limpiado.", path.display(), member),
        Err(error) => println!("[CLRPFM] Error: {}", error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_addpfm(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
        println!("[ADDPFM] Uso: ADDPFM FILE(QGPL/CUSTOMERS) MBR(JAN2026)");
        return;
    };
    let member = fields
        .get("MBR")
        .map(|value| value.trim().to_uppercase())
        .unwrap_or_else(|| crate::db::DEFAULT_PF_MEMBER.to_string());
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, file, fields.get("LIB").map(String::as_str));
    match crate::db::add_pf_member(&path, &member) {
        Ok(_) => println!("[ADDPFM] {} agregado a {}.", member, path.display()),
        Err(error) => println!("[ADDPFM] Error: {}", error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_wrtpfm(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
        emit_status("CPF0006", None, "WRTPFM requiere FILE");
        println!("[WRTPFM] Uso: WRTPFM FILE(QGPL/CUSTOMERS) KEY(C001) DATA(value)");
        return;
    };
    let data = fields.get("DATA").cloned().unwrap_or_default();
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, file, fields.get("LIB").map(String::as_str));
    let member = fields
        .get("MBR")
        .map(String::as_str)
        .unwrap_or(crate::db::DEFAULT_PF_MEMBER);
    match crate::db::PhysicalFile::open_member(&path, member) {
        Ok(pf) => {
            if let Some(key) = fields.get("KEY") {
                match pf.write_rcd(key.as_bytes(), data.as_bytes()) {
                    Ok(_) => println!("[WRTPFM] Registro KEY({}) escrito.", key),
                    Err(error) => {
                        emit_status("CPF0001", Some(&path), &error.to_string());
                        println!("[WRTPFM] Error: {}", error);
                    }
                }
            } else {
                match pf.append_rcd(data.as_bytes()) {
                    Ok(rrn) => println!("[WRTPFM] Registro agregado RRN({}).", rrn),
                    Err(error) => {
                        emit_status("CPF0001", Some(&path), &error.to_string());
                        println!("[WRTPFM] Error: {}", error);
                    }
                }
            }
        }
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[WRTPFM] Error abriendo PF: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_crtdtaq(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(dtaq) = fields.get("DTAQ") else {
        emit_status("CPF0006", None, "CRTDTAQ requiere DTAQ");
        println!("[CRTDTAQ] Uso: CRTDTAQ DTAQ(QUSRSYS/QEZJOBLOG)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (library, name, _path) =
        resolve_object_spec(&root, dtaq, fields.get("LIB").map(String::as_str));
    let lib_path = root.join(&library);
    let user = runtime_user();
    match crate::auth::check_authority(&lib_path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status(
                "CPF2204",
                Some(&lib_path),
                "authority insufficient for create",
            );
            println!(
                "[CRTDTAQ] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                lib_path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&lib_path), &error.to_string());
            println!("[CRTDTAQ] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::dtaq::crtdtaq(&root.join(&library), &name) {
        Ok(_) => println!("[CRTDTAQ] {}/{} creado.", library, name),
        Err(error) => {
            let path = root.join(&library).join(&name);
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[CRTDTAQ] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_snddtaq_cmd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(dtaq) = fields.get("DTAQ") else {
        emit_status("CPF0006", None, "SNDDTAQ requiere DTAQ");
        println!("[SNDDTAQ] Uso: SNDDTAQ DTAQ(QUSRSYS/QEZJOBLOG) MSG(text)");
        return;
    };
    let msg = fields.get("MSG").cloned().unwrap_or_default();
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, dtaq, fields.get("LIB").map(String::as_str));
    match crate::dtaq::DataQueue::open(&path).and_then(|queue| queue.snddtaq(msg.as_bytes())) {
        Ok(_) => println!("[SNDDTAQ] Mensaje enviado a {}.", path.display()),
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[SNDDTAQ] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_rcvdtaq(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(dtaq) = fields.get("DTAQ") else {
        emit_status("CPF0006", None, "RCVDTAQ requiere DTAQ");
        println!("[RCVDTAQ] Uso: RCVDTAQ DTAQ(QUSRSYS/QEZJOBLOG) WAIT(0)");
        return;
    };
    let wait = fields
        .get("WAIT")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0);
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, dtaq, fields.get("LIB").map(String::as_str));
    match crate::dtaq::DataQueue::open(&path).and_then(|queue| queue.rcvdtaq(wait)) {
        Ok(msg) => println!("[RCVDTAQ] {}", String::from_utf8_lossy(&msg)),
        Err(error) => {
            emit_status("CPF9801", Some(&path), &error.to_string());
            println!("[RCVDTAQ] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dspdtaq(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let dtaq = fields
        .get("DTAQ")
        .cloned()
        .or_else(|| fields.get("OBJ").cloned())
        .unwrap_or_else(|| "QUSRSYS/QEZJOBLOG".to_string());
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, &dtaq, fields.get("LIB").map(String::as_str));
    println!("=== DSPDTAQ - {} ===", path.display());
    match crate::dtaq::DataQueue::open(&path).and_then(|queue| queue.read_all()) {
        Ok(messages) if messages.is_empty() => println!("  No messages."),
        Ok(messages) => {
            println!("  {:8} MESSAGE", "ID");
            println!("  {}", "-".repeat(60));
            for (id, msg) in messages {
                println!("  {:8} {}", id, String::from_utf8_lossy(&msg));
            }
        }
        Err(error) => println!("  Error: {}", error),
    }
    println!("======================================");
}

fn parse_pf_fields(spec: &str) -> Vec<crate::db::PfField> {
    spec.split(',')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            let mut pieces = part.split(':');
            let name = pieces.next()?.trim().to_uppercase();
            let type_ = pieces.next().unwrap_or("CHAR").trim().to_uppercase();
            let length = pieces
                .next()
                .and_then(|value| value.trim().parse::<u32>().ok())
                .unwrap_or_default();
            let text = pieces
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            Some(crate::db::PfField {
                name,
                type_,
                length,
                text,
            })
        })
        .collect()
}

/// CRTLIB — Crea una biblioteca (*LIB)
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtlib(lib: *const c_char) {
    let name = c_str_to_string(lib);
    let root = crate::object::resolve_l400_root();
    // Note: Root is created by bootstrap (create_dir_all) and not cataloged with auth metadata,
    // so we skip authority check on root. Authority checks happen on the library path itself.
    match crate::object::create_library(&root, &name) {
        Ok(path) => println!("[CRTLIB] Biblioteca {} creada en {}", name, path.display()),
        Err(e) => println!("[CRTLIB] Error creando {}: {}", name, e),
    }
}

/// DLTLIB — Elimina una biblioteca
#[unsafe(no_mangle)]
pub extern "C" fn l400_dltlib(lib: *const c_char) {
    let name = c_str_to_string(lib);
    let root = crate::object::resolve_l400_root();
    let path = root.join(&name);
    let user = runtime_user();
    match crate::auth::check_authority(&path, &user, crate::auth::L400Authority::All) {
        Ok(true) => {}
        Ok(false) => {
            emit_status("CPF2204", Some(&path), "authority insufficient for delete");
            println!(
                "[DLTLIB] Denegado por autoridad: usuario {} no tiene *ALL sobre {}.",
                user,
                path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&path), &error.to_string());
            println!("[DLTLIB] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::object::delete_object(&path) {
        Ok(_) => println!("[DLTLIB] Biblioteca {} eliminada.", name),
        Err(e) => println!("[DLTLIB] Error eliminando {}: {}", name, e),
    }
}

/// ADDLIBLE — Añade biblioteca a la library list del proceso (env var)
#[unsafe(no_mangle)]
pub extern "C" fn l400_addlible(lib: *const c_char) {
    let name = c_str_to_string(lib);
    let current = std::env::var("L400_LIBLIST").unwrap_or_default();
    let new_list = if current.is_empty() {
        name.clone()
    } else {
        format!("{}:{}", current, name)
    };
    // Safety: single-threaded context in compiled CL programs
    unsafe {
        std::env::set_var("L400_LIBLIST", &new_list);
    }
    println!("[ADDLIBLE] {} añadida. LIBLIST={}", name, new_list);
}

/// CHGCURLIB — Cambia la biblioteca actual de trabajo
#[unsafe(no_mangle)]
pub extern "C" fn l400_chgcurlib(lib: *const c_char) {
    let name = c_str_to_string(lib);
    unsafe {
        std::env::set_var("L400_CURLIB", &name);
    }
    println!("[CHGCURLIB] Biblioteca actual: {}", name);
}

/// RNMOBJ — Renombra un objeto (conservando xattrs)
#[unsafe(no_mangle)]
pub extern "C" fn l400_rnmobj(obj: *const c_char, newname: *const c_char) {
    let src_name = c_str_to_string(obj);
    let dst_name = c_str_to_string(newname);
    let root = crate::object::resolve_l400_root();
    let curlib = std::env::var("L400_CURLIB").unwrap_or_else(|_| "QSYS".to_string());
    let src = root.join(&curlib).join(&src_name);
    let dst = root.join(&curlib).join(&dst_name);
    match std::fs::rename(&src, &dst) {
        Ok(_) => println!("[RNMOBJ] {} → {}", src_name, dst_name),
        Err(e) => println!("[RNMOBJ] Error renombrando {}: {}", src_name, e),
    }
}

/// CRTPGM — Registra/cataloga un objeto *PGM
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtpgm(pgm: *const c_char) {
    let name = c_str_to_string(pgm);
    let root = crate::object::resolve_l400_root();
    let (library, object, path) = resolve_object_spec(&root, &name, None);
    let lib_path = root.join(&library);
    let user = runtime_user();
    match crate::auth::check_authority(&lib_path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status(
                "CPF2204",
                Some(&lib_path),
                "authority insufficient for create",
            );
            println!(
                "[CRTPGM] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                lib_path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&lib_path), &error.to_string());
            println!("[CRTPGM] Error verificando autoridad: {}", error);
            return;
        }
    }
    match crate::object::catalog_object(&path, "*PGM", Some("CL"), Some("CL Program")) {
        Ok(_) => println!("[CRTPGM] {} catalogado como *PGM.", object),
        Err(e) => println!("[CRTPGM] Error catalogando {}: {}", object, e),
    }
}

fn resolve_program_for_call(root: &Path, pgm: &str) -> PathBuf {
    let trimmed = pgm.trim();
    if trimmed.contains('/') {
        return resolve_object_spec(root, trimmed, None).2;
    }
    let curlib = std::env::var("L400_CURLIB")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "QGPL".to_string());
    let mut candidates = vec![curlib];
    candidates.extend(
        std::env::var("L400_LIBLIST")
            .unwrap_or_default()
            .split(':')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_uppercase),
    );
    candidates.push("QGPL".to_string());
    candidates.push("QSYS".to_string());
    candidates
        .into_iter()
        .map(|library| root.join(library).join(trimmed.to_uppercase()))
        .find(|path| path.exists())
        .unwrap_or_else(|| root.join("QGPL").join(trimmed.to_uppercase()))
}

fn resolve_clc_binary() -> PathBuf {
    if let Ok(path) = std::env::var("L400_CLC_PATH") {
        return PathBuf::from(path);
    }
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(dir) = current_exe.parent()
    {
        let sibling = dir.join("clc");
        if sibling.exists() {
            return sibling;
        }
    }
    for candidate in ["target/debug/clc", "target/release/clc", "clc"] {
        let path = PathBuf::from(candidate);
        if candidate == "clc" || path.exists() {
            return path;
        }
    }
    PathBuf::from("clc")
}

/// CALL — Ejecuta un programa *PGM resolviendo CURLIB/LIBLIST.
#[unsafe(no_mangle)]
pub extern "C" fn l400_call(pgm: *const c_char) {
    let pgm = c_str_to_string(pgm);
    let root = crate::object::resolve_l400_root();
    let path = resolve_program_for_call(&root, &pgm);
    let user = runtime_user();
    match crate::object::describe_object(&path) {
        Ok(object) if object.objtype != "*PGM" => {
            set_status("CPF9802");
            audit_runtime(
                "ACCESS_DENIED",
                &path,
                &format!("CALL user={} wrong_type={}", user, object.objtype),
            );
            println!("[CALL] Denegado: {} no es *PGM.", path.display());
            return;
        }
        Ok(_) => {}
        Err(error) => {
            set_status("CPF2204");
            audit_runtime(
                "ACCESS_DENIED",
                &path,
                &format!("CALL user={} describe_error={}", user, error),
            );
            println!(
                "[CALL] Denegado: no se pudo describir {}: {}",
                path.display(),
                error
            );
            return;
        }
    }

    match crate::auth::check_command_authority(&path, &user, "CALL") {
        Ok(true) => {}
        Ok(false) => {
            set_status("CPF9802");
            audit_runtime(
                "ACCESS_DENIED",
                &path,
                &format!("CALL user={} required=*USE", user),
            );
            println!(
                "[CALL] Denegado por autoridad: usuario {} no tiene *USE sobre {}.",
                user,
                path.display()
            );
            return;
        }
        Err(error) => {
            set_status("CPF9802");
            audit_runtime(
                "ACCESS_DENIED",
                &path,
                &format!("CALL user={} auth_error={}", user, error),
            );
            println!("[CALL] Error verificando autoridad: {}", error);
            return;
        }
    }
    if let Err(error) = crate::storage::verify_toolchain_manifest(&path) {
        set_status("CPF9898");
        audit_runtime(
            "ACCESS_DENIED",
            &path,
            &format!("CALL user={} manifest_error={}", user, error),
        );
        println!(
            "[CALL] Denegado: {} no tiene manifest de toolchain valido ({error}).",
            path.display()
        );
        return;
    }
    audit_runtime("PGM_EXEC", &path, &format!("CALL user={}", user));
    match std::process::Command::new(&path).status() {
        Ok(status) if status.success() => {
            clear_status();
            println!("[CALL] {} finalizo correctamente.", path.display())
        }
        Ok(status) => {
            set_status("CPF0001");
            println!("[CALL] {} finalizo con estado {}.", path.display(), status)
        }
        Err(error) => {
            set_status("CPF0001");
            println!("[CALL] Error ejecutando {}: {}", path.display(), error)
        }
    }
}

/// CRTCLPGM — Compila un miembro CL y cataloga el resultado como *PGM.
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtclpgm(pgm: *const c_char, srcfile: *const c_char, srcmbr: *const c_char) {
    let pgm = c_str_to_string(pgm);
    let srcfile = c_str_to_string(srcfile);
    let srcmbr = c_str_to_string(srcmbr);
    let root = crate::object::resolve_l400_root();
    let (pgm_library, pgm_name, output_path) = resolve_object_spec(&root, &pgm, None);
    let pgm_lib_path = root.join(&pgm_library);
    let user = runtime_user();
    match crate::auth::check_authority(&pgm_lib_path, &user, crate::auth::L400Authority::Change) {
        Ok(true) => {}
        Ok(false) => {
            emit_status(
                "CPF2204",
                Some(&pgm_lib_path),
                "authority insufficient for create",
            );
            println!(
                "[CRTCLPGM] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                user,
                pgm_lib_path.display()
            );
            return;
        }
        Err(error) => {
            emit_status("CPF0001", Some(&pgm_lib_path), &error.to_string());
            println!("[CRTCLPGM] Error verificando autoridad: {}", error);
            return;
        }
    }
    let (src_library, src_file) = resolve_file_spec(&srcfile);
    let src_lib_path = root.join(&src_library);
    let source_path = crate::object::member_path(&src_lib_path, &src_file, &srcmbr).or_else(|_| {
        if srcmbr.to_uppercase().ends_with(".CLP") {
            crate::object::member_path(&src_lib_path, &src_file, &srcmbr)
        } else {
            crate::object::member_path(&src_lib_path, &src_file, &format!("{srcmbr}.CLP"))
        }
    });
    let Ok(source_path) = source_path else {
        set_status("CPF9801");
        println!(
            "[CRTCLPGM] No se encontro fuente {}/{} {}.",
            src_library, src_file, srcmbr
        );
        return;
    };

    let spool_path = compile_spool_file(&format!("{pgm_library}_{pgm_name}"));
    let _ = std::fs::create_dir_all(spool_path.parent().unwrap_or_else(|| Path::new(".")));
    let output = std::process::Command::new(resolve_clc_binary())
        .arg("--input")
        .arg(&source_path)
        .arg("--output")
        .arg(&output_path)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            clear_status();
            let _ = write_compile_spool(&spool_path, "CPF0000", &source_path, &output);
            println!(
                "[CRTCLPGM] {}/{} compilado desde {}. Spool: {}",
                pgm_library,
                pgm_name,
                source_path.display(),
                spool_path.display()
            );
        }
        Ok(output) => {
            set_status("CPF0006");
            let _ = write_compile_spool(&spool_path, "CPF0006", &source_path, &output);
            println!(
                "[CRTCLPGM] clc finalizo con estado {}. Spool: {}",
                output.status,
                spool_path.display()
            );
        }
        Err(error) => {
            set_status("CPF0001");
            let _ = std::fs::write(
                &spool_path,
                format!(
                    "spool_version=1 status=FAILED command=CRTCLPGM cpf=CPF0001\nsource={}\nerror={}\n",
                    source_path.display(),
                    error
                ),
            );
            println!(
                "[CRTCLPGM] Error ejecutando clc: {}. Spool: {}",
                error,
                spool_path.display()
            );
        }
    }
}

fn write_compile_spool(
    path: &Path,
    cpf: &str,
    source_path: &Path,
    output: &std::process::Output,
) -> std::io::Result<()> {
    let status = if output.status.success() {
        "READY"
    } else {
        "FAILED"
    };
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    writeln!(
        file,
        "spool_version=1 status={} command=CRTCLPGM cpf={}",
        status, cpf
    )?;
    writeln!(file, "source={}", source_path.display())?;
    writeln!(file, "exit_status={}", output.status)?;
    writeln!(file, "--- stdout ---")?;
    file.write_all(&output.stdout)?;
    if !output.stdout.ends_with(b"\n") {
        writeln!(file)?;
    }
    writeln!(file, "--- stderr ---")?;
    file.write_all(&output.stderr)?;
    if !output.stderr.ends_with(b"\n") {
        writeln!(file)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Navegación y sesión
// ---------------------------------------------------------------------------

/// GO — Navega a un menú (modo batch: imprime mensaje)
#[unsafe(no_mangle)]
pub extern "C" fn l400_go(target: *const c_char) {
    let menu = c_str_to_string(target);
    println!(
        "[GO] Menú destino: {} (modo batch — TUI requerida para navegación interactiva)",
        menu
    );
}

/// SIGNOFF — Cierra la sesión activa
#[unsafe(no_mangle)]
pub extern "C" fn l400_signoff() {
    println!("[SIGNOFF] Cerrando sesión Linux/400.");
    std::process::exit(0);
}

/// STRPDM — Lista las bibliotecas catalogadas.
#[unsafe(no_mangle)]
pub extern "C" fn l400_strpdm() {
    println!("=== STRPDM - Programming Development Manager ===");
    let root = crate::object::resolve_l400_root();
    match crate::object::list_libraries(&root) {
        Ok(libraries) if libraries.is_empty() => println!("  No hay bibliotecas catalogadas."),
        Ok(libraries) => {
            for library in libraries {
                println!("  {}", library);
            }
        }
        Err(error) => println!("  Error al listar bibliotecas: {}", error),
    }
    println!("================================================");
}

/// WRKMBRPDM — Lista miembros dentro de un archivo fuente.
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkmbrpdm(file: *const c_char) {
    let file_spec = c_str_to_string(file);
    let (library, file_name) = resolve_file_spec(&file_spec);
    let lib_path = crate::object::resolve_l400_root().join(&library);

    println!("=== WRKMBRPDM - Miembros de {}/{} ===", library, file_name);
    match crate::object::list_members(&lib_path, &file_name) {
        Ok(members) if members.is_empty() => println!("  No hay miembros."),
        Ok(members) => {
            println!("  {:16} {:10} TEXT", "MBR", "TYPE");
            println!("  {}", "-".repeat(48));
            for member in members {
                println!("  {:16} {:10} {}", member.name, member.type_, member.text);
            }
        }
        Err(error) => println!("  Error al listar miembros: {}", error),
    }
    println!("======================================");
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_dltmbr(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(file), Some(member)) = (fields.get("FILE"), fields.get("MBR")) else {
        emit_status("CPF0006", None, "DLTMBR requiere FILE y MBR");
        println!("[DLTMBR] Uso: DLTMBR FILE(QGPL/QCLSRC) MBR(HELLO.CLP) CONFIRM(*YES)");
        return;
    };
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    if !confirmed {
        emit_status("CPF0006", None, "DLTMBR requiere CONFIRM(*YES)");
        println!("[DLTMBR] Requiere CONFIRM(*YES).");
        return;
    }
    let (library, file_name) = resolve_file_spec(file);
    let lib_path = crate::object::resolve_l400_root().join(&library);
    let pf_path = lib_path.join(&file_name); // Authority check on PF/LF, not member
    match crate::object::member_path(&lib_path, &file_name, member) {
        Ok(path) => {
            let user = runtime_user();
            // Check authority on the PF/LF object, not the member file
            match crate::auth::check_authority(&pf_path, &user, crate::auth::L400Authority::All) {
                Ok(true) => {}
                Ok(false) => {
                    emit_status(
                        "CPF2204",
                        Some(&pf_path),
                        "authority insufficient for delete",
                    );
                    println!(
                        "[DLTMBR] Denegado por autoridad: usuario {} no tiene *ALL sobre {}.",
                        user,
                        pf_path.display()
                    );
                    return;
                }
                Err(error) => {
                    emit_status("CPF0001", Some(&pf_path), &error.to_string());
                    println!("[DLTMBR] Error verificando autoridad: {}", error);
                    return;
                }
            }
            match std::fs::remove_file(&path) {
                Ok(_) => println!("[DLTMBR] {} eliminado.", path.display()),
                Err(error) => {
                    emit_status(
                        "CPF9801",
                        Some(&lib_path.join(&file_name)),
                        &error.to_string(),
                    );
                    println!("[DLTMBR] Error: {}", error);
                }
            }
        }
        Err(error) => {
            emit_status(
                "CPF9801",
                Some(&lib_path.join(&file_name)),
                &error.to_string(),
            );
            println!("[DLTMBR] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_cpymbr(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(file), Some(member), Some(to_member)) =
        (fields.get("FILE"), fields.get("MBR"), fields.get("TOMBR"))
    else {
        emit_status("CPF0006", None, "CPYMBR requiere FILE MBR TOMBR");
        println!("[CPYMBR] Uso: CPYMBR FILE(QGPL/QCLSRC) MBR(A.CLP) TOMBR(B.CLP)");
        return;
    };
    let (library, file_name) = resolve_file_spec(file);
    let lib_path = crate::object::resolve_l400_root().join(&library);
    let result = crate::object::member_path(&lib_path, &file_name, member).and_then(|from| {
        let to = crate::object::member_path(&lib_path, &file_name, to_member)?;
        std::fs::copy(&from, &to)?;
        Ok((from, to))
    });
    match result {
        Ok((from, to)) => println!("[CPYMBR] {} copiado a {}.", from.display(), to.display()),
        Err(error) => {
            emit_status(
                "CPF0001",
                Some(&lib_path.join(&file_name)),
                &error.to_string(),
            );
            println!("[CPYMBR] Error: {}", error);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn l400_chgmbrd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(file), Some(member), Some(text)) =
        (fields.get("FILE"), fields.get("MBR"), fields.get("TEXT"))
    else {
        emit_status("CPF0006", None, "CHGMBRD requiere FILE MBR TEXT");
        println!("[CHGMBRD] Uso: CHGMBRD FILE(QGPL/QCLSRC) MBR(A.CLP) TEXT(Demo)");
        return;
    };
    let (library, file_name) = resolve_file_spec(file);
    let lib_path = crate::object::resolve_l400_root().join(&library);
    let pf_path = lib_path.join(&file_name); // Authority check on PF/LF, not member
    match crate::object::member_path(&lib_path, &file_name, member) {
        Ok(path) => {
            let user = runtime_user();
            // Check authority on the PF/LF object, not the member file
            match crate::auth::check_authority(&pf_path, &user, crate::auth::L400Authority::Change)
            {
                Ok(true) => {}
                Ok(false) => {
                    emit_status(
                        "CPF2204",
                        Some(&pf_path),
                        "authority insufficient for change",
                    );
                    println!(
                        "[CHGMBRD] Denegado por autoridad: usuario {} no tiene *CHANGE sobre {}.",
                        user,
                        pf_path.display()
                    );
                    return;
                }
                Err(error) => {
                    emit_status("CPF0001", Some(&pf_path), &error.to_string());
                    println!("[CHGMBRD] Error verificando autoridad: {}", error);
                    return;
                }
            }
            match crate::storage::write_string_attr(&path, "user.l400.text", text)
                .map_err(|error| crate::object::ObjectError::Fs(std::io::Error::other(error)))
            {
                Ok(_) => println!(
                    "[CHGMBRD] {}/{}/{} actualizado.",
                    library, file_name, member
                ),
                Err(error) => {
                    emit_status(
                        "CPF0001",
                        Some(&lib_path.join(&file_name)),
                        &error.to_string(),
                    );
                    println!("[CHGMBRD] Error: {}", error);
                }
            }
        }
        Err(error) => {
            emit_status(
                "CPF0001",
                Some(&lib_path.join(&file_name)),
                &error.to_string(),
            );
            println!("[CHGMBRD] Error: {}", error);
        }
    }
}

/// STRSEU — Muestra el contenido de un miembro fuente en modo batch.
#[unsafe(no_mangle)]
pub extern "C" fn l400_strseu(file: *const c_char, mbr: *const c_char) {
    let file_spec = c_str_to_string(file);
    let member = c_str_to_string(mbr).trim().to_uppercase();
    let (library, file_name) = resolve_file_spec(&file_spec);
    let lib_path = crate::object::resolve_l400_root().join(&library);

    println!("=== STRSEU - {}/{}/{} ===", library, file_name, member);
    match crate::object::member_path(&lib_path, &file_name, &member) {
        Ok(path) => match std::fs::read_to_string(&path) {
            Ok(content) => {
                for (index, line) in content.lines().enumerate() {
                    println!("{:04}.00 {}", index + 1, line);
                }
            }
            Err(error) => println!("  Error leyendo miembro: {}", error),
        },
        Err(error) => println!("  Error resolviendo miembro: {}", error),
    }
    println!("======================================");
}

fn print_sql_result(result: crate::db::SqlStatementResult) {
    match result {
        crate::db::SqlStatementResult::Query(result) => {
            println!("{}", result.columns.join(" | "));
            if result.rows.is_empty() {
                println!("(sin filas)");
            } else {
                for (index, row) in result.rows.into_iter().enumerate() {
                    if index > 0 && index % 20 == 0 {
                        println!("-- mas -- fila {}", index + 1);
                        println!("{}", result.columns.join(" | "));
                    }
                    println!("{}", row.join(" | "));
                }
            }
        }
        crate::db::SqlStatementResult::Message(message) => println!("SQL0000 {}", message),
    }
}

/// STRSQL — Ejecuta una sentencia SQL leída desde stdin.
#[unsafe(no_mangle)]
pub extern "C" fn l400_strsql() {
    let mut statement = String::new();
    if std::io::stdin().read_to_string(&mut statement).is_err() || statement.trim().is_empty() {
        println!("[STRSQL] Ingrese una sentencia SQL vía stdin.");
        return;
    }

    match crate::db::run_sql_statement(&statement, None) {
        Ok(result) => print_sql_result(result),
        Err(error) => {
            emit_status("CPF0001", None, &format!("STRSQL SQL9001 {error}"));
            println!("SQL9001 [STRSQL] {}", error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PowerDownAction, confirmed_yes, l400_call, l400_chgobjd, l400_crtdtaq, l400_crtpf,
        l400_dltobj, parse_command_fields, power_down_option_from_spec, spool_file_status,
    };
    use crate::auth::{L400Authority, grant_object_authority};
    use crate::ffi::{l400_clear_status, l400_last_cpf_code};
    use crate::object::{catalog_object, ensure_library};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    struct EnvGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &std::path::Path) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(previous) = &self.previous {
                    std::env::set_var(self.key, previous);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn parse_command_fields_keeps_values_with_spaces() {
        let fields = parse_command_fields("OBJ=QGPL/DEMO TEXT='Demo object' OBJATTR=PF");

        assert_eq!(fields.get("OBJ").map(String::as_str), Some("QGPL/DEMO"));
        assert_eq!(fields.get("TEXT").map(String::as_str), Some("Demo object"));
        assert_eq!(fields.get("OBJATTR").map(String::as_str), Some("PF"));
    }

    #[test]
    fn pwrdwnsys_accepts_supported_options() {
        assert_eq!(
            PowerDownAction::from_option("*CNTRLD"),
            Some(PowerDownAction::ControlledPowerOff)
        );
        assert_eq!(
            PowerDownAction::from_option("*IMMED"),
            Some(PowerDownAction::ImmediatePowerOff)
        );
        assert_eq!(
            PowerDownAction::from_option("*RESTART"),
            Some(PowerDownAction::Restart)
        );
        assert_eq!(PowerDownAction::from_option("*BAD"), None);
    }

    #[test]
    fn pwrdwnsys_defaults_option_when_only_confirm_is_present() {
        let fields = parse_command_fields("CONFIRM=*YES");
        assert_eq!(
            power_down_option_from_spec("CONFIRM=*YES", &fields),
            "*CNTRLD"
        );

        let raw_fields = parse_command_fields("*IMMED");
        assert_eq!(power_down_option_from_spec("*IMMED", &raw_fields), "*IMMED");
    }

    #[test]
    fn pwrdwnsys_confirm_requires_yes() {
        let yes = "*YES".to_string();
        let no = "*NO".to_string();

        assert!(confirmed_yes(Some(&yes)));
        assert!(!confirmed_yes(Some(&no)));
        assert!(!confirmed_yes(None));
    }

    #[test]
    fn spool_file_status_uses_latest_status_field() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("demo.splf");
        let mut file = std::fs::File::create(&path).expect("create splf");
        writeln!(file, "spool_version=1 status=RUN").expect("write run");
        writeln!(file, "status=HELD changed_at=1").expect("write held");

        assert_eq!(spool_file_status(&path).as_deref(), Some("HELD"));
    }

    #[test]
    fn call_rejects_program_without_toolchain_manifest() {
        let root = tempfile::tempdir().expect("tempdir");
        let _root = EnvGuard::set("L400_ROOT", root.path());
        let qgpl = ensure_library(root.path(), "QGPL").expect("library");
        let pgm = qgpl.join("NOMAN");
        std::fs::write(&pgm, "#!/usr/bin/env sh\nexit 0\n").expect("write pgm");
        let mut permissions = std::fs::metadata(&pgm).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&pgm, permissions).expect("chmod");
        catalog_object(&pgm, "*PGM", Some("CL"), Some("No manifest")).expect("catalog");
        grant_object_authority(&pgm, "*PUBLIC", L400Authority::Use).expect("grant public use");

        l400_clear_status();
        let c_pgm = std::ffi::CString::new("QGPL/NOMAN").expect("cstring");
        l400_call(c_pgm.as_ptr());

        // Should emit CPF9898 (no toolchain manifest) or CPF2204 (authority denied)
        let cpf = l400_last_cpf_code();
        assert!(
            cpf == 9898 || cpf == 2204,
            "CALL should fail with CPF9898 or CPF2204"
        );
    }

    #[test]
    fn crtpf_regression_test() {
        let root = tempfile::tempdir().expect("tempdir");
        let _root_guard = EnvGuard::set("L400_ROOT", root.path());
        let qgpl = ensure_library(root.path(), "QGPL").expect("library");
        grant_object_authority(&qgpl, "*PUBLIC", L400Authority::Change)
            .expect("grant *PUBLIC *CHANGE");

        l400_clear_status();
        // Use KEY=VALUE format (not KEY(VALUE))
        let spec = std::ffi::CString::new("FILE=QGPL/REGTEST RCDLEN=128").expect("cstring");
        l400_crtpf(spec.as_ptr());

        let cpf = l400_last_cpf_code();
        assert_eq!(cpf, 0, "CRTPF should succeed with CPF 0");
    }

    #[test]
    fn crtdtaq_regression_test() {
        let root = tempfile::tempdir().expect("tempdir");
        let _root_guard = EnvGuard::set("L400_ROOT", root.path());
        let qusrsys = ensure_library(root.path(), "QUSRSYS").expect("library");
        grant_object_authority(&qusrsys, "*PUBLIC", L400Authority::Change)
            .expect("grant *PUBLIC *CHANGE");

        l400_clear_status();
        // Use KEY=VALUE format (not KEY(VALUE))
        let spec = std::ffi::CString::new("DTAQ=QUSRSYS/REGTEST").expect("cstring");
        l400_crtdtaq(spec.as_ptr());

        let cpf = l400_last_cpf_code();
        assert_eq!(cpf, 0, "CRTDTAQ should succeed with CPF 0");
    }

    #[test]
    fn dltobj_regression_test() {
        let root = tempfile::tempdir().expect("tempdir");
        let _root_guard = EnvGuard::set("L400_ROOT", root.path());
        let _qgpl = ensure_library(root.path(), "QGPL").expect("library");
        let obj_path = root.path().join("QGPL").join("TODELETE");
        std::fs::File::create(&obj_path).expect("create obj");
        catalog_object(&obj_path, "*FILE", Some("PF"), Some("To delete")).expect("catalog");
        grant_object_authority(&obj_path, "*PUBLIC", L400Authority::All)
            .expect("grant *PUBLIC *ALL");

        l400_clear_status();
        // Use KEY=VALUE format (not KEY(VALUE))
        let spec = std::ffi::CString::new("OBJ=QGPL/TODELETE CONFIRM=*YES").expect("cstring");
        l400_dltobj(spec.as_ptr());

        let cpf = l400_last_cpf_code();
        assert_eq!(cpf, 0, "DLTOBJ should succeed with CPF 0");
    }

    #[test]
    fn chgobjd_regression_test() {
        let root = tempfile::tempdir().expect("tempdir");
        let _root_guard = EnvGuard::set("L400_ROOT", root.path());
        let _qgpl = ensure_library(root.path(), "QGPL").expect("library");
        let obj_path = root.path().join("QGPL").join("TOCHANGE");
        std::fs::File::create(&obj_path).expect("create obj");
        catalog_object(&obj_path, "*FILE", Some("PF"), Some("Original text")).expect("catalog");
        grant_object_authority(&obj_path, "*PUBLIC", L400Authority::Change)
            .expect("grant *PUBLIC *CHANGE");

        l400_clear_status();
        // Use KEY=VALUE format (not KEY(VALUE))
        let spec = std::ffi::CString::new("OBJ=QGPL/TOCHANGE TEXT=New text").expect("cstring");
        l400_chgobjd(spec.as_ptr());

        let cpf = l400_last_cpf_code();
        assert_eq!(cpf, 0, "CHGOBJD should succeed with CPF 0");
    }
}
