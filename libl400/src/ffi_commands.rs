/// Funciones C públicas del runtime Linux/400.
/// Estas son invocadas por los programas CL compilados por `clc`.
/// Cada función implementa la semántica del comando OS/400 correspondiente
/// delegando a los módulos internos de `libl400`.
use std::ffi::CStr;
use std::fs::OpenOptions;
use std::io::Write;
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
pub extern "C" fn l400_savlib(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let parts: Vec<&str> = spec_str.split_whitespace().collect();
    
    let mut library = String::new();
    let mut savf = String::new();
    let mut target = "*LOCAL";
    
    for part in parts {
        if part.starts_with("LIB(") {
            library = part.trim_start_matches("LIB(").trim_end_matches(')').to_string();
        } else if part.starts_with("DEV(") {
        } else if part.starts_with("SAVF(") {
            savf = part.trim_start_matches("SAVF(").trim_end_matches(')').to_string();
        } else if part.starts_with("TARGET(") {
            target = part.trim_start_matches("TARGET(").trim_end_matches(')');
        }
    }
    
    if library.is_empty() || savf.is_empty() {
        emit_status("CPF0001", None, "LIB and SAVF required");
        return;
    }
    
    match crate::backup::savlib(&library, &savf, target) {
        Ok(msg) => {
            emit_status("CPF0000", None, &msg);
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("SAVLIB failed: {}", e));
        }
    }
}

/// RSTLIB - Restore Library from *SAVF (Save File)
/// Usage: RSTLIB LIB(MYLIB) DEV(*SAVF) SAVF(MYLIB_SAV) SOURCE(*LOCAL|*MEGA)
#[unsafe(no_mangle)]
pub extern "C" fn l400_rstlib(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let parts: Vec<&str> = spec_str.split_whitespace().collect();
    
    let mut library = String::new();
    let mut savf = String::new();
    let mut source = "*LOCAL";
    
    for part in parts {
        if part.starts_with("LIB(") {
            library = part.trim_start_matches("LIB(").trim_end_matches(')').to_string();
        } else if part.starts_with("DEV(") {
        } else if part.starts_with("SAVF(") {
            savf = part.trim_start_matches("SAVF(").trim_end_matches(')').to_string();
        } else if part.starts_with("SOURCE(") {
            source = part.trim_start_matches("SOURCE(").trim_end_matches(')');
        }
    }
    
    if library.is_empty() || savf.is_empty() {
        emit_status("CPF0001", None, "LIB and SAVF required");
        return;
    }
    match crate::backup::rstlib(&savf, &library, source) {
        Ok(msg) => {
            emit_status("CPF0000", None, &msg);
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("RSTLIB failed: {}", e));
        }
    }
}

/// SAVOBJ - Save Object to *SAVF
/// Usage: SAVOBJ OBJ(MYOBJ) LIB(MYLIB) DEV(*SAVF) SAVF(MYOBJ_SAV) TARGET(*LOCAL|*MEGA)
#[unsafe(no_mangle)]
pub extern "C" fn l400_savobj(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let parts: Vec<&str> = spec_str.split_whitespace().collect();
    
    let mut object = String::new();
    let mut library = String::new();
    let mut savf = String::new();
    let mut target = "*LOCAL";
    
    for part in parts {
        if part.starts_with("OBJ(") {
            object = part.trim_start_matches("OBJ(").trim_end_matches(')').to_string();
        } else if part.starts_with("LIB(") {
            library = part.trim_start_matches("LIB(").trim_end_matches(')').to_string();
        } else if part.starts_with("DEV(") {
        } else if part.starts_with("SAVF(") {
            savf = part.trim_start_matches("SAVF(").trim_end_matches(')').to_string();
        } else if part.starts_with("TARGET(") {
            target = part.trim_start_matches("TARGET(").trim_end_matches(')');
        }
    }
    
    if object.is_empty() || library.is_empty() || savf.is_empty() {
        emit_status("CPF0001", None, "OBJ, LIB and SAVF required");
        return;
    }
    
    match crate::backup::savobj(&object, &library, &savf, target) {
        Ok(msg) => {
            emit_status("CPF0000", None, &msg);
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("SAVOBJ failed: {}", e));
        }
    }
}

/// CHKOBJINT - Check Object Integrity after Restore
/// Usage: CHKOBJINT OBJ(LIB/OBJ)
#[unsafe(no_mangle)]
pub extern "C" fn l400_chkobjint(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let object = spec_str.trim_start_matches("OBJ(").trim_end_matches(')');
    
    match crate::backup::chkobjint(object) {
        Ok(result) => {
            emit_status("CPF0000", None, &format!("Result . . . . . . . . : {}", result));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("CHKOBJINT failed: {}", e));
        }
    }
}

/// WRKSAVF - Work with Save Files
/// Lists available *SAVF files (local and mega.io if mounted)
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrksavf(_spec: *const c_char) {
    match crate::backup::list_savf() {
        Ok(savfs) => {
            if savfs.is_empty() {
                emit_status("CPF0000", None, "No *SAVF files found");
                return;
            }
            
            let mut output = String::new();
            output.push_str("SAVF NAME       LIBRARY    SIZE     CREATED    DESCRIPTION\n");
            output.push_str("-------------  ---------  --------  ---------  -----------\n");
            
            for savf in savfs {
                output.push_str(&format!("{:<15} {:<10} {:<10} {:<10} {}\n",
                    savf.name, savf.library, savf.size, savf.created, savf.description));
            }
            
            emit_status("CPF0000", None, &output);
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("WRKSAVF failed: {}", e));
        }
    }
}

// ============================================================================
// Missing stub functions referenced by l400cmd.rs
// ============================================================================

/// DSPPOLICY - Display Policy (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_dsppolicy() {
    println!("=== DSPPOLICY - Display Policy ===");
    println!("Policy: default");
    println!("Status: active");
}

/// DSPAUD - Display Auditing (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_dspaudit() {
    println!("=== DSPAUD - Display Auditing ===");
    println!("Auditing: enabled");
    println!("Log: /var/log/l400");
}

/// CRTLIB - Create Library (stub - actual implementation might be elsewhere)
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtlib(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Library created (stub)");
}

/// DLTLIB - Delete Library (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_dltlib(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Library deleted (stub)");
}

/// ADDLIBLE - Add Library List Entry (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_addlible(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Library list entry added (stub)");
}

/// CHGCURLIB - Change Current Library (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_chgcurlib(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Current library changed (stub)");
}

/// CRTPGM - Create Program (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtpgm(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Program created (stub)");
}

/// CALL - Call Program (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_call(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Program called (stub)");
}

/// STRPDM - Start PDM (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_strpdm() {
    println!("=== STRPDM - Start PDM ===");
    println!("PDM started (stub)");
}

/// WRKMBRPDM - Work with Members (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrkmbrpdm(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Work with members (stub)");
}

/// DLTMBR - Delete Member (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_dltmbr(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Member deleted (stub)");
}

/// CPYMBR - Copy Member (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_cpymbr(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Member copied (stub)");
}

/// CHGMBRD - Change Member Description (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_chgmbrd(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Member description changed (stub)");
}

/// CRTPF - Create Physical File (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtpf(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Physical file created (stub)");
}

/// CRTLF - Create Logical File (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtlf(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Logical file created (stub)");
}

/// DSPPFM - Display Physical File Member (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_dsppfm(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Display physical file member (stub)");
}

/// CLRPFM - Clear Physical File Member (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_clrpfm(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Physical file member cleared (stub)");
}

// ============================================================================
// Missing stub functions referenced by l400cmd.rs
// ============================================================================

/// ADDFFM - Add Physical File Member (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_addpfm(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Member added (stub)");
}

/// WRTPFM - Write to Physical File Member (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_wrtpfm(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Member written (stub)");
}

/// CRTPF - Create Physical File (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtdtaq(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Data queue created (stub)");
}

/// SNDDTAQ - Send Data Queue (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_snddtaq_cmd(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Data sent to queue (stub)");
}

/// RCVDTAQ - Receive Data Queue (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_rcvdtaq(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Data received from queue (stub)");
}

/// DSPDTAQ - Display Data Queue (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_dspdtaq(spec: *const c_char) {
    let _ = c_str_to_string(spec);
    emit_status("CPF0000", None, "Display data queue (stub)");
}

/// RNMOBJ - Rename Object (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_rnmobj(arg1: *const c_char, arg2: *const c_char) {
    let _ = c_str_to_string(arg1);
    let _ = c_str_to_string(arg2);
    emit_status("CPF0000", None, "Object renamed (stub)");
}

/// SIGNOFF - Sign Off (stub)
#[unsafe(no_mangle)]
pub extern "C" fn l400_signoff() {
    println!("=== SIGNOFF - Sign Off ===");
    println!("User signed off (stub)");
}
/// GO - Go to Menu (corrected - one string argument)
#[unsafe(no_mangle)]
pub extern "C" fn l400_go(target: *const c_char) {
    let _ = c_str_to_string(target);
    println!("=== GO - Go to Menu ===");
    println!("Menu displayed (stub)");
}

// ============================================================================

// ============================================================================
// Phase 5: User Profile Commands
// ============================================================================

/// CRTUSRPRF - Create User Profile
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtusrprf(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let usrprf_name = fields.get("USRPRF").map(|s| s.as_str()).unwrap_or("");
    let text = fields.get("TEXT").map(|s| s.as_str()).unwrap_or("");
    
    if usrprf_name.is_empty() {
        emit_status("CPF0001", None, "USRPRF parameter required");
        return;
    }
    
    // Check authorization
    let path = crate::usrprf::get_usrprf_path(usrprf_name);
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&path, &current_user, "CRTUSRPRF") {
        Ok(true) => {
            match crate::usrprf::create_user_profile(usrprf_name, Some(text)) {
                Ok(_) => {
                    emit_status("CPF0000", None, &format!("User profile {} created", usrprf_name));
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Create failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to create user profile {}", usrprf_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// CHGUSRPRF - Change User Profile
#[unsafe(no_mangle)]
pub extern "C" fn l400_chgusrprf(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let usrprf_name = fields.get("USRPRF").map(|s| s.as_str()).unwrap_or("");
    let text = fields.get("TEXT").map(|s| s.as_str());
    let status = fields.get("STATUS").map(|s| s.as_str());
    let password = fields.get("PASSWORD").map(|s| s.as_str());
    let home_library = fields.get("HOMELIB").map(|s| s.as_str());
    let current_library = fields.get("CURRLIB").map(|s| s.as_str());
    let group_profiles = fields.get("GRPPRF").map(|s| s.as_str());
    
    if usrprf_name.is_empty() {
        emit_status("CPF0001", None, "USRPRF parameter required");
        return;
    }
    
    // Check authorization
    let path = crate::usrprf::get_usrprf_path(usrprf_name);
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&path, &current_user, "CHGUSRPRF") {
        Ok(true) => {
            match crate::usrprf::change_user_profile(
                usrprf_name,
                text,
                status,
                password,
                home_library,
                current_library,
                group_profiles,
            ) {
                Ok(_) => {
                    emit_status("CPF0000", None, &format!("User profile {} changed", usrprf_name));
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Change failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to change user profile {}", usrprf_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// DLTUSRPRF - Delete User Profile
#[unsafe(no_mangle)]
pub extern "C" fn l400_dltusrprf(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let usrprf_name = fields.get("USRPRF").map(|s| s.as_str()).unwrap_or("");
    
    if usrprf_name.is_empty() {
        emit_status("CPF0001", None, "USRPRF parameter required");
        return;
    }
    
    // Check authorization
    let path = crate::usrprf::get_usrprf_path(usrprf_name);
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&path, &current_user, "DLTUSRPRF") {
        Ok(true) => {
            // Check if we should keep the system user
            let keep_system = fields.get("OWNSOBJ").map(|s| s == "*KEEP").unwrap_or(false);
            
            match crate::usrprf::delete_user_profile(usrprf_name, keep_system) {
                Ok(_) => {
                    emit_status("CPF0000", None, &format!("User profile {} deleted", usrprf_name));
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Delete failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to delete user profile {}", usrprf_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// DSPUSRPRF - Display User Profile
#[unsafe(no_mangle)]
pub extern "C" fn l400_dspusrprf(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let usrprf_name = fields.get("USRPRF").map(|s| s.as_str()).unwrap_or("");
    
    if usrprf_name.is_empty() {
        emit_status("CPF0001", None, "USRPRF parameter required");
        return;
    }
    
    // Check authorization
    let path = crate::usrprf::get_usrprf_path(usrprf_name);
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&path, &current_user, "DSPUSRPRF") {
        Ok(true) => {
            match crate::usrprf::display_user_profile(usrprf_name) {
                Ok(info) => {
                    let mut output = String::new();
                    output.push_str(&format!("User profile . . . . . . . . . : {}\n", info.name));
                    output.push_str(&format!("Description . . . . . . . . . : {}\n", info.description));
                    output.push_str(&format!("Status . . . . . . . . . . . : {}\n", info.status));
                    output.push_str(&format!("User ID  . . . . . . . . . . : {}\n", info.uid));
                    output.push_str(&format!("Owner . . . . . . . . . . . : {}\n", info.owner));
                    output.push_str(&format!("Creation date . . . . . . . . : {}\n", info.creation_date));
                    
                    if let Some(ref lib) = info.home_library {
                        output.push_str(&format!("Home library  . . . . . . . : {}\n", lib));
                    }
                    if let Some(ref lib) = info.current_library {
                        output.push_str(&format!("Current library . . . . . . : {}\n", lib));
                    }
                    if !info.group_profiles.is_empty() {
                        output.push_str(&format!("Group profiles . . . . . . .: {}\n", info.group_profiles.join(", ")));
                    }
                    
                    emit_status("CPF0000", None, &output);
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Display failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to display user profile {}", usrprf_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// USRPRF_CHANGE - Change own user profile (wrapper for CHGUSRPRF)
#[unsafe(no_mangle)]
pub extern "C" fn l400_usrprf_change(spec: *const c_char) {
    l400_chgusrprf(spec);
}

/// CRTCLPGM - Create CL Program (wrapper for l400_crtpgm with 3 args)
/// This function is called by compiled CL programs
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtclpgm(pgm: *const c_char, srcfile: *const c_char, srcmbr: *const c_char) {
    // Combine the three arguments into a single spec string for l400_crtpgm
    let pgm_str = c_str_to_string(pgm);
    let srcfile_str = c_str_to_string(srcfile);
    let srcmbr_str = c_str_to_string(srcmbr);
    
    let spec = format!("PGM({}) SRCFILE({}) SRCMBR({})", pgm_str, srcfile_str, srcmbr_str);
    l400_crtpgm(spec.as_ptr() as *const c_char);
}

/// STRSEU - Start SEU (2 args: file, member)
#[unsafe(no_mangle)]
pub extern "C" fn l400_strseu(arg1: *const c_char, arg2: *const c_char) {
    let _ = c_str_to_string(arg1);
    let _ = c_str_to_string(arg2);
    println!("=== STRSEU - Start SEU ===");
    println!("SEU started (stub)");
}

/// STRSQL - Start SQL (no arguments)
#[unsafe(no_mangle)]
pub extern "C" fn l400_strsql() {
    println!("=== STRSQL - Start SQL ===");
    println!("SQL started (stub)");
}

// ============================================================================
// Phase 6: Job Queue Commands (CRTJOBQ, DLTJOBQ, HLDJOBQ, RLSJOBQ)
// ============================================================================

/// CRTJOBQ - Create Job Queue
#[unsafe(no_mangle)]
pub extern "C" fn l400_crtjobq(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let jobq_name = fields.get("JOBQ").map(|s| s.as_str()).unwrap_or("");
    let text = fields.get("TEXT").map(|s| s.as_str()).unwrap_or("Job Queue");
    let subsystem = fields.get("SBS").map(|s| s.as_str()).unwrap_or("QBATCH");
    let max_active = fields.get("MAXACT").map(|s| s.as_str()).unwrap_or("1");
    let priority = fields.get("PRIORITY").map(|s| s.as_str()).unwrap_or("5");
    
    if jobq_name.is_empty() {
        emit_status("CPF0001", None, "JOBQ parameter required");
        return;
    }
    
    // Check authorization
    let path = crate::object::resolve_l400_root().join("QSYS").join(format!("{}.JOBQ", jobq_name.to_uppercase()));
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&path, &current_user, "CRTJOBQ") {
        Ok(true) => {
            match crate::object::create_object_with_metadata(
                &crate::object::resolve_l400_root().join("QSYS"),
                jobq_name,
                "*JOBQ",
                Some("JOBQ"),
                Some(text),
            ) {
                Ok(_) => {
                    // Set job queue attributes
                    let jobq_path = crate::object::resolve_l400_root()
                        .join("QSYS")
                        .join(format!("{}.JOBQ", jobq_name.to_uppercase()));
                    
                    crate::storage::write_string_attr(&jobq_path, crate::storage::L400_JOBQ_STATUS_ATTR, "*ACTIVE")
                        .ok();
                    crate::storage::write_string_attr(&jobq_path, crate::storage::L400_JOBQ_SUBSYSTEM_ATTR, subsystem)
                        .ok();
                    crate::storage::write_string_attr(&jobq_path, crate::storage::L400_JOBQ_MAX_ACTIVE_ATTR, max_active)
                        .ok();
                    crate::storage::write_string_attr(&jobq_path, crate::storage::L400_JOBQ_PRIORITY_ATTR, priority)
                        .ok();
                    
                    // Log creation
                    crate::audit::audit_event(
                        "JOBQ_CREATE",
                        &current_user,
                        &jobq_path,
                        &format!("Job queue {} created", jobq_name)
                    ).ok();
                    
                    emit_status("CPF0000", None, &format!("Job queue {} created", jobq_name));
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Create failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to create job queue {}", jobq_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// DLTJOBQ - Delete Job Queue
#[unsafe(no_mangle)]
pub extern "C" fn l400_dltjobq(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let jobq_name = fields.get("JOBQ").map(|s| s.as_str()).unwrap_or("");
    
    if jobq_name.is_empty() {
        emit_status("CPF0001", None, "JOBQ parameter required");
        return;
    }
    
    let jobq_path = crate::object::resolve_l400_root()
        .join("QSYS")
        .join(format!("{}.JOBQ", jobq_name.to_uppercase()));
    
    if !jobq_path.exists() {
        emit_status("CPF0001", None, &format!("Job queue {} not found", jobq_name));
        return;
    }
    
    // Check authorization
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&jobq_path, &current_user, "DLTJOBQ") {
        Ok(true) => {
            // Check if there are active jobs in this queue
            let jobs = crate::cgroup::list_jobs_at(&jobq_path).unwrap_or_default();
            let active_jobs: Vec<_> = jobs.iter().filter(|j| matches!(j.status, crate::cgroup::JobStatus::JobQ | crate::cgroup::JobStatus::Active | crate::cgroup::JobStatus::Held)).collect();
            
            if !active_jobs.is_empty() {
                emit_status("CPF0001", None, &format!("Job queue {} has {} active/held jobs", jobq_name, active_jobs.len()));
                return;
            }
            
            // Delete the job queue object
            match std::fs::remove_file(&jobq_path) {
                Ok(_) => {
                    // Log deletion
                    crate::audit::audit_event(
                        "JOBQ_DELETE",
                        &current_user,
                        &jobq_path,
                        &format!("Job queue {} deleted", jobq_name)
                    ).ok();
                    
                    emit_status("CPF0000", None, &format!("Job queue {} deleted", jobq_name));
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Delete failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to delete job queue {}", jobq_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// HLDJOBQ - Hold Job Queue
#[unsafe(no_mangle)]
pub extern "C" fn l400_hldjobq(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let jobq_name = fields.get("JOBQ").map(|s| s.as_str()).unwrap_or("");
    
    if jobq_name.is_empty() {
        emit_status("CPF0001", None, "JOBQ parameter required");
        return;
    }
    
    let jobq_path = crate::object::resolve_l400_root()
        .join("QSYS")
        .join(format!("{}.JOBQ", jobq_name.to_uppercase()));
    
    if !jobq_path.exists() {
        emit_status("CPF0001", None, &format!("Job queue {} not found", jobq_name));
        return;
    }
    
    // Check authorization
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&jobq_path, &current_user, "HLDJOBQ") {
        Ok(true) => {
            // Set status to *HLD
            match crate::storage::write_string_attr(&jobq_path, crate::storage::L400_JOBQ_STATUS_ATTR, "*HLD") {
                Ok(_) => {
                    // Hold all jobs in this queue
                    if let Ok(jobs) = crate::cgroup::list_jobs_at(&jobq_path) {
                        for job in jobs {
                            if matches!(job.status, crate::cgroup::JobStatus::JobQ | crate::cgroup::JobStatus::Active) {
                                let _ = crate::cgroup::hold_job(job.pid);
                            }
                        }
                    }
                    
                    // Log hold
                    crate::audit::audit_event(
                        "JOBQ_HOLD",
                        &current_user,
                        &jobq_path,
                        &format!("Job queue {} held", jobq_name)
                    ).ok();
                    
                    emit_status("CPF0000", None, &format!("Job queue {} held", jobq_name));
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Hold failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to hold job queue {}", jobq_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// RLSJOBQ - Release Job Queue
#[unsafe(no_mangle)]
pub extern "C" fn l400_rlsjobq(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let jobq_name = fields.get("JOBQ").map(|s| s.as_str()).unwrap_or("");
    
    if jobq_name.is_empty() {
        emit_status("CPF0001", None, "JOBQ parameter required");
        return;
    }
    
    let jobq_path = crate::object::resolve_l400_root()
        .join("QSYS")
        .join(format!("{}.JOBQ", jobq_name.to_uppercase()));
    
    if !jobq_path.exists() {
        emit_status("CPF0001", None, &format!("Job queue {} not found", jobq_name));
        return;
    }
    
    // Check authorization
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&jobq_path, &current_user, "RLSJOBQ") {
        Ok(true) => {
            // Set status to *ACTIVE
            match crate::storage::write_string_attr(&jobq_path, crate::storage::L400_JOBQ_STATUS_ATTR, "*ACTIVE") {
                Ok(_) => {
                    // Release all held jobs in this queue
                    if let Ok(jobs) = crate::cgroup::list_jobs_at(&jobq_path) {
                        for job in jobs {
                            if matches!(job.status, crate::cgroup::JobStatus::Held) {
                                let _ = crate::cgroup::release_job(job.pid);
                            }
                        }
                    }
                    
                    // Log release
                    crate::audit::audit_event(
                        "JOBQ_RELEASE",
                        &current_user,
                        &jobq_path,
                        &format!("Job queue {} released", jobq_name)
                    ).ok();
                    
                    emit_status("CPF0000", None, &format!("Job queue {} released", jobq_name));
                }
                Err(e) => {
                    emit_status("CPF0001", None, &format!("Release failed: {}", e));
                }
            }
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to release job queue {}", jobq_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}
    

/// DSPOUTQ - Display Output Queue
#[unsafe(no_mangle)]
pub extern "C" fn l400_dspoutq(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let outq_name = fields.get("OUTQ").map(|s| s.as_str()).unwrap_or("QPRINT");
    
    println!("=== DSPOUTQ - Display Output Queue {} ===", outq_name);
    
    let outq_path = crate::object::resolve_l400_root()
        .join("QSYS")
        .join(format!("{}.OUTQ", outq_name.to_uppercase()));
    
    if !outq_path.exists() {
        println!("Output queue {} not found.", outq_name);
        println!("=============================");
        return;
    }
    
    // Read output queue attributes
    let retention = crate::storage::read_u32_attr(&outq_path, crate::storage::L400_OUTQ_RETENTION_DAYS_ATTR)
        .ok()
        .flatten()
        .unwrap_or(7);
    
    let routing = crate::storage::read_string_attr(&outq_path, crate::storage::L400_OUTQ_ROUTING_ATTR)
        .ok()
        .flatten()
        .unwrap_or_else(|| "QBATCH".to_string());
    
    let default_status = crate::storage::read_string_attr(&outq_path, crate::storage::L400_OUTQ_DEFAULT_STATUS_ATTR)
        .ok()
        .flatten()
        .unwrap_or_else(|| "*READY".to_string());
    
    println!("  Queue: {}", outq_name);
    println!("  Retention: {} days", retention);
    println!("  Routing: {}", routing);
    println!("  Default Status: {}", default_status);
    println!("");
    println!("  Files in queue:");
    
    let spool_dir = spool_dir();
    if let Ok(entries) = std::fs::read_dir(&spool_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("splf") {
                let outq = crate::storage::read_string_attr(&path, crate::storage::L400_SPOOL_OUTQ_ATTR)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "Unknown".to_string());
                
                if outq.to_uppercase() == outq_name.to_uppercase() {
                    let name = path.file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown");
                    
                    let status = crate::storage::read_string_attr(&path, crate::storage::L400_SPOOL_STATUS_ATTR)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "*READY".to_string());
                    
                    println!("    {} - Status: {}", name, status);
                }
            }
        }
    }
    
    println!("=============================");
}

/// HLDQUTQ - Hold Output Queue
#[unsafe(no_mangle)]
pub extern "C" fn l400_hldoutq(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let outq_name = fields.get("OUTQ").map(|s| s.as_str()).unwrap_or("");
    
    if outq_name.is_empty() {
        emit_status("CPF0001", None, "OUTQ parameter required");
        return;
    }
    
    let outq_path = crate::object::resolve_l400_root()
        .join("QSYS")
        .join(format!("{}.OUTQ", outq_name.to_uppercase()));
    
    if !outq_path.exists() {
        emit_status("CPF0001", None, &format!("Output queue {} not found", outq_name));
        return;
    }
    
    // Check authorization
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&outq_path, &current_user, "HLDOUTQ") {
        Ok(true) => {
            // Set status to *HLD
            if let Err(e) = crate::storage::write_string_attr(
                &outq_path,
                crate::storage::L400_OUTQ_DEFAULT_STATUS_ATTR,
                "*HLD"
            ) {
                emit_status("CPF0001", None, &format!("Hold failed: {}", e));
                return;
            }
            
            // Hold all spool files in this queue
            let spool_dir = spool_dir();
            if let Ok(entries) = std::fs::read_dir(&spool_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("splf") {
                        let outq = crate::storage::read_string_attr(&path, crate::storage::L400_SPOOL_OUTQ_ATTR)
                            .ok()
                            .flatten()
                            .unwrap_or_default();
                        
                        if outq.to_uppercase() == outq_name.to_uppercase() {
                            let _ = crate::storage::write_string_attr(
                                &path,
                                crate::storage::L400_SPOOL_STATUS_ATTR,
                                "*HELD"
                            );
                        }
                    }
                }
            }
            
            crate::audit::audit_event(
                "OUTQ_HOLD",
                &current_user,
                &outq_path,
                &format!("Output queue {} held", outq_name)
            ).ok();
            
            emit_status("CPF0000", None, &format!("Output queue {} held", outq_name));
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to hold output queue {}", outq_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

/// RLSOUTQ - Release Output Queue
#[unsafe(no_mangle)]
pub extern "C" fn l400_rlsoutq(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let outq_name = fields.get("OUTQ").map(|s| s.as_str()).unwrap_or("");
    
    if outq_name.is_empty() {
        emit_status("CPF0001", None, "OUTQ parameter required");
        return;
    }
    
    let outq_path = crate::object::resolve_l400_root()
        .join("QSYS")
        .join(format!("{}.OUTQ", outq_name.to_uppercase()));
    
    if !outq_path.exists() {
        emit_status("CPF0001", None, &format!("Output queue {} not found", outq_name));
        return;
    }
    
    // Check authorization
    let current_user = crate::audit::current_l400_user();
    match crate::auth::check_command_authority(&outq_path, &current_user, "RLSOUTQ") {
        Ok(true) => {
            // Set status to *READY
            if let Err(e) = crate::storage::write_string_attr(
                &outq_path,
                crate::storage::L400_OUTQ_DEFAULT_STATUS_ATTR,
                "*READY"
            ) {
                emit_status("CPF0001", None, &format!("Release failed: {}", e));
                return;
            }
            
            // Release all spool files in this queue
            let spool_dir = spool_dir();
            if let Ok(entries) = std::fs::read_dir(&spool_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("splf") {
                        let outq = crate::storage::read_string_attr(&path, crate::storage::L400_SPOOL_OUTQ_ATTR)
                            .ok()
                            .flatten()
                            .unwrap_or_default();
                        
                        if outq.to_uppercase() == outq_name.to_uppercase() {
                            let _ = crate::storage::write_string_attr(
                                &path,
                                crate::storage::L400_SPOOL_STATUS_ATTR,
                                "*READY"
                            );
                        }
                    }
                }
            }
            
            crate::audit::audit_event(
                "OUTQ_RELEASE",
                &current_user,
                &outq_path,
                &format!("Output queue {} released", outq_name)
            ).ok();
            
            emit_status("CPF0000", None, &format!("Output queue {} released", outq_name));
        }
        Ok(false) => {
            emit_status("CPF0001", None, &format!("Not authorized to release output queue {}", outq_name));
        }
        Err(e) => {
            emit_status("CPF0001", None, &format!("Authorization check failed: {}", e));
        }
    }
}

// ============================================================================
// Phase 7: Additional Spool Commands (HLDSPOOL, RLSSPOOL)
// ============================================================================

/// HLDSPOOL - Hold Spool File
#[unsafe(no_mangle)]
pub extern "C" fn l400_hldspool(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let spool_file = fields.get("SPLF").map(|s| s.as_str()).unwrap_or("");
    let job = fields.get("JOB").map(|s| s.as_str()).unwrap_or("");
    
    if spool_file.is_empty() && job.is_empty() {
        emit_status("CPF0001", None, "SPLF or JOB parameter required");
        return;
    }
    
    // Find spool file(s)
    let spool_dir = spool_dir();
    if !spool_dir.exists() {
        emit_status("CPF0001", None, "Spool directory not found");
        return;
    }
    
    let mut found = false;
    if let Ok(entries) = std::fs::read_dir(&spool_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains(spool_file) || name.contains(job) {
                    // Set status to *HELD
                    if let Err(e) = crate::storage::write_string_attr(
                        &path,
                        crate::storage::L400_SPOOL_STATUS_ATTR,
                        "*HELD"
                    ) {
                        emit_status("CPF0001", None, &format!("Hold failed: {}", e));
                        return;
                    }
                    found = true;
                }
            }
        }
    }
    
    if found {
        crate::audit::audit_event(
            "SPOOL_HOLD",
            &crate::audit::current_l400_user(),
            &spool_dir,
            &format!("Spool file held: {}", spool_file)
        ).ok();
        emit_status("CPF0000", None, &format!("Spool file {} held", spool_file));
    } else {
        emit_status("CPF0001", None, &format!("Spool file {} not found", spool_file));
    }
}

/// RLSSPOOL - Release Spool File
#[unsafe(no_mangle)]
pub extern "C" fn l400_rlsspool(spec: *const c_char) {
    let spec_str = c_str_to_string(spec);
    let fields = parse_command_fields(&spec_str);
    
    let spool_file = fields.get("SPLF").map(|s| s.as_str()).unwrap_or("");
    let job = fields.get("JOB").map(|s| s.as_str()).unwrap_or("");
    
    if spool_file.is_empty() && job.is_empty() {
        emit_status("CPF0001", None, "SPLF or JOB parameter required");
        return;
    }
    
    // Find spool file(s)
    let spool_dir = spool_dir();
    if !spool_dir.exists() {
        emit_status("CPF0001", None, "Spool directory not found");
        return;
    }
    
    let mut found = false;
    if let Ok(entries) = std::fs::read_dir(&spool_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains(spool_file) || name.contains(job) {
                    // Set status to *READY
                    if let Err(e) = crate::storage::write_string_attr(
                        &path,
                        crate::storage::L400_SPOOL_STATUS_ATTR,
                        "*READY"
                    ) {
                        emit_status("CPF0001", None, &format!("Release failed: {}", e));
                        return;
                    }
                    found = true;
                }
            }
        }
    }
    
    if found {
        crate::audit::audit_event(
            "SPOOL_RELEASE",
            &crate::audit::current_l400_user(),
            &spool_dir,
            &format!("Spool file released: {}", spool_file)
        ).ok();
        emit_status("CPF0000", None, &format!("Spool file {} released", spool_file));
    } else {
        emit_status("CPF0001", None, &format!("Spool file {} not found", spool_file));
    }
}
