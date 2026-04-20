/// Funciones C públicas del runtime Linux/400.
/// Estas son invocadas por los programas CL compilados por `clc`.
/// Cada función implementa la semántica del comando OS/400 correspondiente
/// delegando a los módulos internos de `libl400`.
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;
use std::path::Path;

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

    // Jobs activos
    let root = crate::object::resolve_l400_root();
    let job_path = root.join("QSYS").join("JOBQ");
    if let Ok(jobs) = crate::cgroup::list_jobs_at(&job_path) {
        println!("  Jobs registrados: {}", jobs.len());
    }

    println!("================================================");
}

/// WRKACTJOB — Lista jobs activos del job registry
#[no_mangle]
pub extern "C" fn l400_wrkactjob() {
    println!("=== WRKACTJOB - Trabajos Activos ===");
    let root = crate::object::resolve_l400_root();
    let job_path = root.join("QSYS").join("JOBQ");
    match crate::cgroup::list_jobs_at(&job_path) {
        Ok(jobs) if jobs.is_empty() => println!("  No hay trabajos activos."),
        Ok(jobs) => {
            println!("  {:20} {:10} {:8}", "JOB", "ESTADO", "PID");
            println!("  {}", "-".repeat(42));
            for j in &jobs {
                println!(
                    "  {:20} {:10} {:8}",
                    j.name,
                    format!("{:?}", j.status),
                    j.pid
                );
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
    let filter = c_str_to_string(usrprf);
    println!(
        "=== WRKUSRPRF - Perfiles de Usuario (filtro: {}) ===",
        filter
    );
    let qsys = Path::new("/l400/QSYS");
    if qsys.exists() {
        if let Ok(entries) = std::fs::read_dir(qsys) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_uppercase();
                if name.ends_with(".USRPRF") {
                    let display = name.trim_end_matches(".USRPRF");
                    if filter == "*ALL" || display.contains(&filter) {
                        println!("  {}", display);
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
    let opt = c_str_to_string(option);
    println!(
        "[PWRDWNSYS] Solicitando parada del sistema (OPTION={})",
        opt
    );
    match opt.as_str() {
        "*IMMED" => {
            println!("[PWRDWNSYS] Parada inmediata — requiere root.");
            // En un entorno real: Command::new("shutdown").arg("-h").arg("now").status()
        }
        "*RESTART" => {
            println!("[PWRDWNSYS] Reinicio del sistema — requiere root.");
            // Command::new("shutdown").arg("-r").arg("now").status()
        }
        _ => {
            println!("[PWRDWNSYS] Parada controlada (*CNTRLD) — no implementada en modo batch.");
        }
    }
}

// ---------------------------------------------------------------------------
// Objetos y bibliotecas
// ---------------------------------------------------------------------------

/// WRKOBJ — Busca y lista objetos del catálogo
#[no_mangle]
pub extern "C" fn l400_wrkobj(obj_filter: *const c_char) {
    let filter = c_str_to_string(obj_filter);
    println!("=== WRKOBJ - Objetos (filtro: {}) ===", filter);
    let root = crate::object::resolve_l400_root();
    let qsys = root.join("QSYS");
    if let Ok(objects) = crate::object::list_objects(&qsys) {
        if objects.is_empty() {
            println!("  No hay objetos en QSYS.");
        } else {
            println!("  {:20} {:10} {:10}", "OBJETO", "TIPO", "ATRIB");
            println!("  {}", "-".repeat(44));
            for obj in &objects {
                println!(
                    "  {:20} {:10} {:10}",
                    obj.name,
                    obj.objtype,
                    obj.attribute.as_deref().unwrap_or("-")
                );
            }
        }
    } else {
        println!("  QSYS no disponible como catálogo.");
    }
    println!("=====================================");
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
    let curlib = std::env::var("L400_CURLIB").unwrap_or_else(|_| "QSYS".to_string());
    let path = root.join(&curlib).join(&name);
    match crate::object::catalog_object(&path, "*PGM", Some("CL"), Some("CL Program")) {
        Ok(_) => println!("[CRTPGM] {} catalogado como *PGM.", name),
        Err(e) => println!("[CRTPGM] Error catalogando {}: {}", name, e),
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

/// STRSQL — Ejecuta una consulta SELECT leída desde stdin.
#[no_mangle]
pub extern "C" fn l400_strsql() {
    let mut statement = String::new();
    if std::io::stdin().read_to_string(&mut statement).is_err() || statement.trim().is_empty() {
        println!("[STRSQL] Ingrese una sentencia SQL vía stdin.");
        return;
    }

    match crate::db::run_select_query(&statement, None) {
        Ok(result) => {
            println!("{}", result.columns.join(" | "));
            if result.rows.is_empty() {
                println!("(sin filas)");
            } else {
                for row in result.rows {
                    println!("{}", row.join(" | "));
                }
            }
        }
        Err(error) => println!("[STRSQL] Error: {}", error),
    }
}
