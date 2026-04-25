/// Funciones C públicas del runtime Linux/400.
/// Estas son invocadas por los programas CL compilados por `clc`.
/// Cada función implementa la semántica del comando OS/400 correspondiente
/// delegando a los módulos internos de `libl400`.
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::process::Command;

fn c_str_to_string(s: *const c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
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
    for token in input.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            fields.insert(key.trim().to_uppercase(), value.trim().to_string());
        }
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

// l400_sndpgmmsg está definida en ffi.rs — no se duplica aquí.

// Gestión de sistema
// ---------------------------------------------------------------------------

/// WRKSYSSTS — Muestra estado del sistema (CPU, jobs, memoria)
#[no_mangle]
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
#[no_mangle]
pub extern "C" fn l400_wrkactjob() {
    l400_wrkactjob_spec(std::ptr::null());
}

#[no_mangle]
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

/// WRKSYSVAL — Muestra valores de configuración del sistema
#[no_mangle]
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
#[no_mangle]
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

/// WRKUSRPRF — Gestiona perfiles de usuario
#[no_mangle]
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
            Ok(_) => println!("  Perfil {} creado.", filter),
            Err(error) => println!("  Error creando perfil {}: {}", filter, error),
        }
        println!("========================================");
        return;
    }

    if matches!(action.as_str(), "*DISABLE" | "DISABLE") {
        let path = qsys.join(&filter);
        match xattr::set(&path, "user.l400.disabled", b"yes") {
            Ok(_) => println!("  Perfil {} desactivado.", filter),
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
                if let Ok(object) = crate::object::describe_object(&path) {
                    if object.objtype == "*USRPRF" && matches_pattern(&name, &filter) {
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
        }
    } else {
        println!("  Directorio QSYS no disponible.");
    }
    println!("====================================================");
}

/// PWRDWNSYS — Apaga o reinicia el sistema
#[no_mangle]
pub extern "C" fn l400_pwrdwnsys(option: *const c_char) {
    let spec = c_str_to_string(option);
    let fields = parse_command_fields(&spec);
    let opt = fields
        .get("OPTION")
        .cloned()
        .unwrap_or_else(|| spec.trim().to_string())
        .to_uppercase();
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    println!(
        "[PWRDWNSYS] Solicitando parada del sistema (OPTION={})",
        opt
    );
    if !confirmed {
        println!("[PWRDWNSYS] Requiere CONFIRM(*YES) para ejecutar una accion real.");
        return;
    }
    if unsafe { libc::geteuid() } != 0 {
        println!("[PWRDWNSYS] Accion real requiere root.");
        return;
    }
    match opt.as_str() {
        "*IMMED" => {
            let _ = Command::new("shutdown").arg("-h").arg("now").status();
        }
        "*RESTART" => {
            let _ = Command::new("shutdown").arg("-r").arg("now").status();
        }
        _ => {
            println!("[PWRDWNSYS] OPTION debe ser *IMMED o *RESTART.");
        }
    }
}

// ---------------------------------------------------------------------------
// Objetos y bibliotecas
// ---------------------------------------------------------------------------

/// WRKOBJ — Busca y lista objetos del catálogo
#[no_mangle]
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

#[no_mangle]
pub extern "C" fn l400_dltobj(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
        println!("[DLTOBJ] Uso: DLTOBJ OBJ(QGPL/MYOBJ) CONFIRM(*YES)");
        return;
    };
    let confirmed = fields
        .get("CONFIRM")
        .map(|value| matches!(value.to_uppercase().as_str(), "*YES" | "YES"))
        .unwrap_or(false);
    if !confirmed {
        println!("[DLTOBJ] Requiere CONFIRM(*YES).");
        return;
    }
    let root = crate::object::resolve_l400_root();
    let (_library, object, path) =
        resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match crate::object::delete_object(&path) {
        Ok(_) => println!("[DLTOBJ] {} eliminado.", object),
        Err(error) => println!("[DLTOBJ] Error eliminando {}: {}", object, error),
    }
}

#[no_mangle]
pub extern "C" fn l400_cpyobj(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(obj), Some(toobj)) = (fields.get("OBJ"), fields.get("TOOBJ")) else {
        println!("[CPYOBJ] Uso: CPYOBJ OBJ(QGPL/A) TOOBJ(QGPL/B)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, src_name, src) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    let (_, dst_name, dst) =
        resolve_object_spec(&root, toobj, fields.get("TOLIB").map(String::as_str));
    match crate::object::copy_object(&src, &dst) {
        Ok(_) => println!("[CPYOBJ] {} copiado a {}.", src_name, dst_name),
        Err(error) => println!("[CPYOBJ] Error copiando {}: {}", src_name, error),
    }
}

#[no_mangle]
pub extern "C" fn l400_dspobjd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
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
            println!("=======================================");
        }
        Err(error) => println!("[DSPOBJD] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_chgobjd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
        println!("[CHGOBJD] Uso: CHGOBJD OBJ(QGPL/MYOBJ) TEXT(Demo)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
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
                Err(error) => println!("[CHGOBJD] Error actualizando objeto: {}", error),
            }
        }
        Err(error) => println!("[CHGOBJD] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_dspobjaut(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(obj) = fields.get("OBJ") else {
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
        Err(error) => println!("[DSPOBJAUT] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_grtobjaut(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(obj), Some(user), Some(aut)) =
        (fields.get("OBJ"), fields.get("USER"), fields.get("AUT"))
    else {
        println!("[GRTOBJAUT] Uso: GRTOBJAUT OBJ(QGPL/MYOBJ) USER(QPGMR) AUT(*USE)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match aut.parse() {
        Ok(authority) => match crate::auth::grant_object_authority(&path, user, authority) {
            Ok(_) => println!("[GRTOBJAUT] Autoridad {} otorgada a {}.", aut, user),
            Err(error) => println!("[GRTOBJAUT] Error: {}", error),
        },
        Err(error) => println!("[GRTOBJAUT] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_rvkobjaut(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(obj), Some(user)) = (fields.get("OBJ"), fields.get("USER")) else {
        println!("[RVKOBJAUT] Uso: RVKOBJAUT OBJ(QGPL/MYOBJ) USER(QPGMR)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (_, _, path) = resolve_object_spec(&root, obj, fields.get("LIB").map(String::as_str));
    match crate::auth::revoke_object_authority(&path, user) {
        Ok(_) => println!("[RVKOBJAUT] Autoridad revocada para {}.", user),
        Err(error) => println!("[RVKOBJAUT] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_crtpf(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
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
        Err(error) => println!("[CRTPF] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_crtlf(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let (Some(file), Some(srcfile)) = (fields.get("FILE"), fields.get("SRCFILE")) else {
        println!("[CRTLF] Uso: CRTLF FILE(QGPL/CUSTBYNAME) SRCFILE(QGPL/CUSTOMERS)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (library, name, _path) =
        resolve_object_spec(&root, file, fields.get("LIB").map(String::as_str));
    let (_src_library, _src_name, src_path) =
        resolve_object_spec(&root, srcfile, fields.get("SRCLIB").map(String::as_str));
    let lib_path = root.join(&library);
    match crate::db::PhysicalFile::open(&src_path)
        .and_then(|pf| crate::db::create_lf(&lib_path, &name, &pf))
    {
        Ok(_) => println!(
            "[CRTLF] {}/{} creado sobre {}.",
            library,
            name,
            src_path.display()
        ),
        Err(error) => println!("[CRTLF] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_dsppfm(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
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
        Err(error) => println!("  Error abriendo PF: {}", error),
    }
    println!("======================================");
}

#[no_mangle]
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

#[no_mangle]
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

#[no_mangle]
pub extern "C" fn l400_wrtpfm(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(file) = fields.get("FILE") else {
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
                    Err(error) => println!("[WRTPFM] Error: {}", error),
                }
            } else {
                match pf.append_rcd(data.as_bytes()) {
                    Ok(rrn) => println!("[WRTPFM] Registro agregado RRN({}).", rrn),
                    Err(error) => println!("[WRTPFM] Error: {}", error),
                }
            }
        }
        Err(error) => println!("[WRTPFM] Error abriendo PF: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_crtdtaq(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(dtaq) = fields.get("DTAQ") else {
        println!("[CRTDTAQ] Uso: CRTDTAQ DTAQ(QUSRSYS/QEZJOBLOG)");
        return;
    };
    let root = crate::object::resolve_l400_root();
    let (library, name, _path) =
        resolve_object_spec(&root, dtaq, fields.get("LIB").map(String::as_str));
    match crate::dtaq::crtdtaq(&root.join(&library), &name) {
        Ok(_) => println!("[CRTDTAQ] {}/{} creado.", library, name),
        Err(error) => println!("[CRTDTAQ] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_snddtaq_cmd(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(dtaq) = fields.get("DTAQ") else {
        println!("[SNDDTAQ] Uso: SNDDTAQ DTAQ(QUSRSYS/QEZJOBLOG) MSG(text)");
        return;
    };
    let msg = fields.get("MSG").cloned().unwrap_or_default();
    let root = crate::object::resolve_l400_root();
    let (_library, _name, path) =
        resolve_object_spec(&root, dtaq, fields.get("LIB").map(String::as_str));
    match crate::dtaq::DataQueue::open(&path).and_then(|queue| queue.snddtaq(msg.as_bytes())) {
        Ok(_) => println!("[SNDDTAQ] Mensaje enviado a {}.", path.display()),
        Err(error) => println!("[SNDDTAQ] Error: {}", error),
    }
}

#[no_mangle]
pub extern "C" fn l400_rcvdtaq(spec: *const c_char) {
    let spec = c_str_to_string(spec);
    let fields = parse_command_fields(&spec);
    let Some(dtaq) = fields.get("DTAQ") else {
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
        Err(error) => println!("[RCVDTAQ] Error: {}", error),
    }
}

#[no_mangle]
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
#[no_mangle]
pub extern "C" fn l400_crtlib(lib: *const c_char) {
    let name = c_str_to_string(lib);
    let root = crate::object::resolve_l400_root();
    match crate::object::create_library(&root, &name) {
        Ok(path) => println!("[CRTLIB] Biblioteca {} creada en {}", name, path.display()),
        Err(e) => println!("[CRTLIB] Error creando {}: {}", name, e),
    }
}

/// DLTLIB — Elimina una biblioteca
#[no_mangle]
pub extern "C" fn l400_dltlib(lib: *const c_char) {
    let name = c_str_to_string(lib);
    let root = crate::object::resolve_l400_root();
    let path = root.join(&name);
    match crate::object::delete_object(&path) {
        Ok(_) => println!("[DLTLIB] Biblioteca {} eliminada.", name),
        Err(e) => println!("[DLTLIB] Error eliminando {}: {}", name, e),
    }
}

/// ADDLIBLE — Añade biblioteca a la library list del proceso (env var)
#[no_mangle]
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
#[no_mangle]
pub extern "C" fn l400_chgcurlib(lib: *const c_char) {
    let name = c_str_to_string(lib);
    unsafe {
        std::env::set_var("L400_CURLIB", &name);
    }
    println!("[CHGCURLIB] Biblioteca actual: {}", name);
}

/// RNMOBJ — Renombra un objeto (conservando xattrs)
#[no_mangle]
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
#[no_mangle]
pub extern "C" fn l400_crtpgm(pgm: *const c_char) {
    let name = c_str_to_string(pgm);
    let root = crate::object::resolve_l400_root();
    let (_library, object, path) = resolve_object_spec(&root, &name, None);
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
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let sibling = dir.join("clc");
            if sibling.exists() {
                return sibling;
            }
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
#[no_mangle]
pub extern "C" fn l400_call(pgm: *const c_char) {
    let pgm = c_str_to_string(pgm);
    let root = crate::object::resolve_l400_root();
    let path = resolve_program_for_call(&root, &pgm);
    match std::process::Command::new(&path).status() {
        Ok(status) if status.success() => {
            println!("[CALL] {} finalizo correctamente.", path.display())
        }
        Ok(status) => println!("[CALL] {} finalizo con estado {}.", path.display(), status),
        Err(error) => println!("[CALL] Error ejecutando {}: {}", path.display(), error),
    }
}

/// CRTCLPGM — Compila un miembro CL y cataloga el resultado como *PGM.
#[no_mangle]
pub extern "C" fn l400_crtclpgm(pgm: *const c_char, srcfile: *const c_char, srcmbr: *const c_char) {
    let pgm = c_str_to_string(pgm);
    let srcfile = c_str_to_string(srcfile);
    let srcmbr = c_str_to_string(srcmbr);
    let root = crate::object::resolve_l400_root();
    let (pgm_library, pgm_name, output_path) = resolve_object_spec(&root, &pgm, None);
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
        println!(
            "[CRTCLPGM] No se encontro fuente {}/{} {}.",
            src_library, src_file, srcmbr
        );
        return;
    };

    let status = std::process::Command::new(resolve_clc_binary())
        .arg("--input")
        .arg(&source_path)
        .arg("--output")
        .arg(&output_path)
        .status();
    match status {
        Ok(status) if status.success() => println!(
            "[CRTCLPGM] {}/{} compilado desde {}.",
            pgm_library,
            pgm_name,
            source_path.display()
        ),
        Ok(status) => println!("[CRTCLPGM] clc finalizo con estado {}.", status),
        Err(error) => println!("[CRTCLPGM] Error ejecutando clc: {}", error),
    }
}

// ---------------------------------------------------------------------------
// Navegación y sesión
// ---------------------------------------------------------------------------

/// GO — Navega a un menú (modo batch: imprime mensaje)
#[no_mangle]
pub extern "C" fn l400_go(target: *const c_char) {
    let menu = c_str_to_string(target);
    println!(
        "[GO] Menú destino: {} (modo batch — TUI requerida para navegación interactiva)",
        menu
    );
}

/// SIGNOFF — Cierra la sesión activa
#[no_mangle]
pub extern "C" fn l400_signoff() {
    println!("[SIGNOFF] Cerrando sesión Linux/400.");
    std::process::exit(0);
}

/// STRPDM — Lista las bibliotecas catalogadas.
#[no_mangle]
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
#[no_mangle]
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

/// STRSEU — Muestra el contenido de un miembro fuente en modo batch.
#[no_mangle]
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
#[no_mangle]
pub extern "C" fn l400_strsql() {
    let mut statement = String::new();
    if std::io::stdin().read_to_string(&mut statement).is_err() || statement.trim().is_empty() {
        println!("[STRSQL] Ingrese una sentencia SQL vía stdin.");
        return;
    }

    match crate::db::run_sql_statement(&statement, None) {
        Ok(result) => print_sql_result(result),
        Err(error) => println!("SQL9001 [STRSQL] {}", error),
    }
}
