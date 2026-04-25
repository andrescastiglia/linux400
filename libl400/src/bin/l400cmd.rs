use std::env;
use std::ffi::CString;
use std::io::{stdin, stdout, IsTerminal};
use std::os::raw::c_char;
use std::path::Path;
use std::process::Command;
use std::process::ExitCode;

use l400::ffi_commands;

const COMMAND_BINARIES: &[&str] = &[
    "WRKSYSSTS",
    "WRKACTJOB",
    "WRKSYSVAL",
    "DSPLOG",
    "WRKUSRPRF",
    "PWRDWNSYS",
    "SBMJOB",
    "WRKOBJ",
    "DLTOBJ",
    "CPYOBJ",
    "DSPOBJD",
    "CHGOBJD",
    "DSPOBJAUT",
    "GRTOBJAUT",
    "RVKOBJAUT",
    "CRTLIB",
    "DLTLIB",
    "ADDLIBLE",
    "CHGCURLIB",
    "RNMOBJ",
    "CRTPGM",
    "GO",
    "SIGNOFF",
    "STRPDM",
    "STRSEU",
    "STRSQL",
    "WRKMBRPDM",
    "CRTPF",
    "CRTLF",
    "DSPPFM",
    "CLRPFM",
    "ADDPFM",
    "WRTPFM",
    "CRTDTAQ",
    "SNDDTAQ",
    "RCVDTAQ",
    "DSPDTAQ",
];

fn main() -> ExitCode {
    let invocation = match Invocation::from_env() {
        Ok(invocation) => invocation,
        Err(code) => return code,
    };

    dispatch(&invocation.command, &invocation.args)
}

struct Invocation {
    command: String,
    args: Vec<String>,
}

impl Invocation {
    fn from_env() -> Result<Self, ExitCode> {
        let mut args = env::args().collect::<Vec<_>>();
        let argv0 = args
            .first()
            .and_then(|value| Path::new(value).file_name())
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "l400cmd".to_string());
        args.remove(0);

        let program = argv0.trim().to_uppercase();
        let command = if program == "L400CMD" {
            match args.first() {
                Some(command) => {
                    let command = command.trim().to_uppercase();
                    args.remove(0);
                    command
                }
                None => {
                    print_usage(None);
                    return Err(ExitCode::from(2));
                }
            }
        } else {
            program
        };

        Ok(Self { command, args })
    }
}

fn dispatch(command: &str, args: &[String]) -> ExitCode {
    match command {
        "WRKSYSSTS" => {
            ffi_commands::l400_wrksyssts();
            ExitCode::SUCCESS
        }
        "WRKACTJOB" => dispatch_spec(
            args,
            &["SBS", "SUBSYSTEM", "STATUS", "OPTION", "PID", "JOB"],
            ffi_commands::l400_wrkactjob_spec,
        ),
        "WRKSYSVAL" => {
            ffi_commands::l400_wrksysval();
            ExitCode::SUCCESS
        }
        "DSPLOG" => {
            ffi_commands::l400_dsplog();
            ExitCode::SUCCESS
        }
        "WRKUSRPRF" => dispatch_spec(args, &["USRPRF", "ACTION"], ffi_commands::l400_wrkusrprf),
        "PWRDWNSYS" => dispatch_spec(args, &["OPTION", "CONFIRM"], ffi_commands::l400_pwrdwnsys),
        "WRKOBJ" => dispatch_spec(
            args,
            &["OBJ", "FILTER", "OBJTYPE", "LIB"],
            ffi_commands::l400_wrkobj,
        ),
        "DLTOBJ" => dispatch_spec(
            args,
            &["OBJ", "OBJTYPE", "LIB", "CONFIRM"],
            ffi_commands::l400_dltobj,
        ),
        "CPYOBJ" => dispatch_spec(
            args,
            &["OBJ", "TOOBJ", "LIB", "TOLIB", "OBJTYPE"],
            ffi_commands::l400_cpyobj,
        ),
        "DSPOBJD" => dispatch_spec(args, &["OBJ", "OBJTYPE", "LIB"], ffi_commands::l400_dspobjd),
        "CHGOBJD" => dispatch_spec(
            args,
            &["OBJ", "OBJTYPE", "LIB", "TEXT", "OBJATTR"],
            ffi_commands::l400_chgobjd,
        ),
        "DSPOBJAUT" => dispatch_spec(
            args,
            &["OBJ", "LIB", "OBJTYPE"],
            ffi_commands::l400_dspobjaut,
        ),
        "GRTOBJAUT" => dispatch_spec(
            args,
            &["OBJ", "LIB", "OBJTYPE", "USER", "AUT"],
            ffi_commands::l400_grtobjaut,
        ),
        "RVKOBJAUT" => dispatch_spec(
            args,
            &["OBJ", "LIB", "OBJTYPE", "USER"],
            ffi_commands::l400_rvkobjaut,
        ),
        "CRTLIB" => dispatch_unary_required(
            command,
            args,
            &["LIB", "NAME"],
            "CRTLIB LIB(QGPL)",
            ffi_commands::l400_crtlib,
        ),
        "DLTLIB" => dispatch_unary_required(
            command,
            args,
            &["LIB", "NAME"],
            "DLTLIB LIB(QGPL)",
            ffi_commands::l400_dltlib,
        ),
        "ADDLIBLE" => dispatch_unary_required(
            command,
            args,
            &["LIB"],
            "ADDLIBLE LIB(QGPL)",
            ffi_commands::l400_addlible,
        ),
        "CHGCURLIB" => dispatch_unary_required(
            command,
            args,
            &["LIB", "CURLIB"],
            "CHGCURLIB LIB(QGPL)",
            ffi_commands::l400_chgcurlib,
        ),
        "RNMOBJ" => dispatch_rnmobj(args),
        "CRTPGM" => dispatch_unary_required(
            command,
            args,
            &["PGM", "NAME"],
            "CRTPGM PGM(MYPGM)",
            ffi_commands::l400_crtpgm,
        ),
        "GO" => dispatch_go(command, args),
        "SIGNOFF" => dispatch_signoff(),
        "STRPDM" => {
            ffi_commands::l400_strpdm();
            ExitCode::SUCCESS
        }
        "STRSEU" => dispatch_strseu(args),
        "STRSQL" => dispatch_strsql(args),
        "WRKMBRPDM" => dispatch_unary_required(
            command,
            args,
            &["FILE"],
            "WRKMBRPDM FILE(QGPL/QCLSRC)",
            ffi_commands::l400_wrkmbrpdm,
        ),
        "SBMJOB" => dispatch_sbmjob(args),
        "CRTPF" => dispatch_spec(
            args,
            &["FILE", "LIB", "RCDLEN", "FIELDS", "KEY", "TEXT"],
            ffi_commands::l400_crtpf,
        ),
        "CRTLF" => dispatch_spec(
            args,
            &["FILE", "LIB", "SRCFILE", "SRCLIB", "KEY", "TEXT"],
            ffi_commands::l400_crtlf,
        ),
        "DSPPFM" => dispatch_spec(args, &["FILE", "LIB", "MBR"], ffi_commands::l400_dsppfm),
        "CLRPFM" => dispatch_spec(
            args,
            &["FILE", "LIB", "MBR", "CONFIRM"],
            ffi_commands::l400_clrpfm,
        ),
        "ADDPFM" => dispatch_spec(args, &["FILE", "LIB", "MBR"], ffi_commands::l400_addpfm),
        "WRTPFM" => dispatch_spec(
            args,
            &["FILE", "LIB", "MBR", "KEY", "DATA"],
            ffi_commands::l400_wrtpfm,
        ),
        "CRTDTAQ" => dispatch_spec(args, &["DTAQ", "LIB"], ffi_commands::l400_crtdtaq),
        "SNDDTAQ" => dispatch_spec(
            args,
            &["DTAQ", "LIB", "MSG"],
            ffi_commands::l400_snddtaq_cmd,
        ),
        "RCVDTAQ" => dispatch_spec(args, &["DTAQ", "LIB", "WAIT"], ffi_commands::l400_rcvdtaq),
        "DSPDTAQ" => dispatch_spec(args, &["DTAQ", "OBJ", "LIB"], ffi_commands::l400_dspdtaq),
        _ => {
            eprintln!("ERROR: comando Linux/400 no reconocido: {command}");
            print_usage(Some(command));
            ExitCode::from(2)
        }
    }
}

fn dispatch_spec(
    args: &[String],
    keys: &[&str],
    callback: extern "C" fn(*const c_char),
) -> ExitCode {
    let spec = command_spec(args, keys);
    call_with_cstring(&spec, callback)
}

fn dispatch_sbmjob(args: &[String]) -> ExitCode {
    let cmd = extract_named_arg(args, &["CMD"]).or_else(|| {
        positional_args(args, &["CMD", "JOB", "JOBQ"])
            .first()
            .cloned()
    });
    let Some(cmd) = cmd else {
        eprintln!("ERROR: SBMJOB requiere CMD(...).");
        eprintln!("Uso: SBMJOB CMD(WRKSYSSTS) JOB(MYJOB) JOBQ(QBATCH)");
        return ExitCode::from(2);
    };
    let job = extract_named_arg(args, &["JOB"]).unwrap_or_else(|| "QBATCH".to_string());
    let jobq = extract_named_arg(args, &["JOBQ"]).unwrap_or_else(|| "QBATCH".to_string());
    match Command::new("sbmjob")
        .arg("--job")
        .arg(job)
        .arg("--jobq")
        .arg(jobq)
        .arg(cmd)
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("ERROR: no se pudo ejecutar sbmjob: {error}");
            ExitCode::from(1)
        }
    }
}

fn dispatch_unary_required(
    command: &str,
    args: &[String],
    keys: &[&str],
    usage: &str,
    callback: extern "C" fn(*const c_char),
) -> ExitCode {
    let value =
        extract_named_arg(args, keys).or_else(|| positional_args(args, keys).first().cloned());
    match value {
        Some(value) => call_with_cstring(&value, callback),
        None => {
            eprintln!("ERROR: faltan parámetros para {command}.");
            eprintln!("Uso: {usage}");
            ExitCode::from(2)
        }
    }
}

fn dispatch_rnmobj(args: &[String]) -> ExitCode {
    let keys = ["OBJ", "NEWNAME"];
    let positional = positional_args(args, &keys);
    let current = extract_named_arg(args, &["OBJ"]).or_else(|| positional.first().cloned());
    let new_name = extract_named_arg(args, &["NEWNAME"]).or_else(|| positional.get(1).cloned());
    let (Some(current), Some(new_name)) = (current, new_name) else {
        eprintln!("ERROR: RNMOBJ requiere el nombre actual y el nuevo nombre.");
        eprintln!("Uso: RNMOBJ OBJ(OLDPGM) NEWNAME(NEWPGM)");
        return ExitCode::from(2);
    };

    call_with_two_cstrings(&current, &new_name, ffi_commands::l400_rnmobj)
}

fn dispatch_strseu(args: &[String]) -> ExitCode {
    let keys = ["FILE", "MBR"];
    let positional = positional_args(args, &keys);
    let file = extract_named_arg(args, &["FILE"]).or_else(|| positional.first().cloned());
    let member = extract_named_arg(args, &["MBR"]).or_else(|| positional.get(1).cloned());
    let (Some(file), Some(member)) = (file, member) else {
        eprintln!("ERROR: STRSEU requiere FILE y MBR.");
        eprintln!("Uso: STRSEU FILE(QGPL/QCLSRC) MBR(HELLO.CLP)");
        return ExitCode::from(2);
    };

    call_with_two_cstrings(&file, &member, ffi_commands::l400_strseu)
}

fn dispatch_strsql(args: &[String]) -> ExitCode {
    if args.is_empty() {
        ffi_commands::l400_strsql();
        return ExitCode::SUCCESS;
    }

    let statement = args.join(" ");
    match l400::run_sql_statement(&statement, None) {
        Ok(l400::SqlStatementResult::Query(result)) => {
            print_query_result(result);
            ExitCode::SUCCESS
        }
        Ok(l400::SqlStatementResult::Message(message)) => {
            println!("SQL0000 {}", message);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SQL9001 [STRSQL] {error}");
            ExitCode::from(1)
        }
    }
}

fn print_query_result(result: l400::QueryResult) {
    let page_size = env::var("L400_SQL_PAGE_SIZE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(20);
    let widths = result
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            result
                .rows
                .iter()
                .filter_map(|row| row.get(index))
                .map(String::len)
                .max()
                .unwrap_or(0)
                .max(column.len())
                .min(32)
        })
        .collect::<Vec<_>>();
    print_row(&result.columns, &widths);
    if result.rows.is_empty() {
        println!("(sin filas)");
        return;
    }
    for (index, row) in result.rows.iter().enumerate() {
        if index > 0 && index % page_size == 0 {
            println!("-- mas -- fila {}", index + 1);
            print_row(&result.columns, &widths);
        }
        print_row(row, &widths);
    }
}

fn print_row(row: &[String], widths: &[usize]) {
    let cells = row
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let width = widths.get(index).copied().unwrap_or(16);
            let mut value = value.clone();
            if value.len() > width {
                value.truncate(width);
            }
            format!("{value:width$}")
        })
        .collect::<Vec<_>>();
    println!("{}", cells.join(" | "));
}

fn dispatch_go(command: &str, args: &[String]) -> ExitCode {
    let target = extract_named_arg(args, &["MENU"])
        .or_else(|| positional_args(args, &["MENU"]).first().cloned());
    let Some(target) = target else {
        eprintln!("ERROR: faltan parámetros para {command}.");
        eprintln!("Uso: GO MAIN");
        return ExitCode::from(2);
    };

    if target.trim().eq_ignore_ascii_case("MAIN") {
        return launch_main_menu();
    }

    call_with_cstring(&target, ffi_commands::l400_go)
}

fn launch_main_menu() -> ExitCode {
    if !stdin().is_terminal() || !stdout().is_terminal() {
        return call_with_cstring("MAIN", ffi_commands::l400_go);
    }

    match Command::new("os400-tui").status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("ERROR: no se pudo lanzar os400-tui: {error}");
            ExitCode::from(1)
        }
    }
}

fn dispatch_signoff() -> ExitCode {
    if stdin().is_terminal() && stdout().is_terminal() {
        let parent_pid = unsafe { libc::getppid() };
        if parent_pid > 1 {
            let parent_name = std::fs::read_to_string(format!("/proc/{parent_pid}/comm"))
                .ok()
                .map(|value| value.trim().to_string())
                .unwrap_or_default();
            if matches!(
                parent_name.as_str(),
                "sh" | "ash" | "bash" | "dash" | "busybox"
            ) {
                println!("[SIGNOFF] Cerrando sesión Linux/400.");
                let result = unsafe { libc::kill(parent_pid, libc::SIGHUP) };
                if result == 0 {
                    return ExitCode::SUCCESS;
                }
            }
        }
    }

    ffi_commands::l400_signoff();
    ExitCode::SUCCESS
}

fn call_with_cstring(value: &str, callback: extern "C" fn(*const c_char)) -> ExitCode {
    match CString::new(value) {
        Ok(value) => {
            callback(value.as_ptr());
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("ERROR: los parámetros no pueden contener bytes NUL.");
            ExitCode::from(2)
        }
    }
}

fn call_with_two_cstrings(
    first: &str,
    second: &str,
    callback: extern "C" fn(*const c_char, *const c_char),
) -> ExitCode {
    let first = match CString::new(first) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("ERROR: los parámetros no pueden contener bytes NUL.");
            return ExitCode::from(2);
        }
    };
    let second = match CString::new(second) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("ERROR: los parámetros no pueden contener bytes NUL.");
            return ExitCode::from(2);
        }
    };

    callback(first.as_ptr(), second.as_ptr());
    ExitCode::SUCCESS
}

fn extract_named_arg(args: &[String], keys: &[&str]) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        let trimmed = arg.trim();
        for key in keys {
            if let Some(value) = parse_named_arg(trimmed, key) {
                return Some(value);
            }
            if trimmed.eq_ignore_ascii_case(key) {
                return args.get(index + 1).cloned();
            }
        }
    }
    None
}

fn command_spec(args: &[String], keys: &[&str]) -> String {
    let mut parts = Vec::new();
    for key in keys {
        if let Some(value) = extract_named_arg(args, &[*key]) {
            let normalized_key = if *key == "FILTER" { "OBJ" } else { key };
            parts.push(format!("{}={}", normalized_key.to_uppercase(), value));
        }
    }

    let positionals = positional_args(args, keys);
    if !positionals.is_empty() && !parts.iter().any(|part| part.starts_with("OBJ=")) {
        parts.push(format!("OBJ={}", positionals[0]));
    }

    parts.join(" ")
}

fn parse_named_arg(token: &str, key: &str) -> Option<String> {
    if token.len() > key.len() + 2
        && token[..key.len()].eq_ignore_ascii_case(key)
        && token[key.len()..].starts_with('(')
        && token.ends_with(')')
    {
        return Some(token[key.len() + 1..token.len() - 1].trim().to_string());
    }

    if token.len() > key.len() + 1
        && token[..key.len()].eq_ignore_ascii_case(key)
        && token[key.len()..].starts_with('=')
    {
        return Some(token[key.len() + 1..].trim().to_string());
    }

    None
}

fn positional_args(args: &[String], keys: &[&str]) -> Vec<String> {
    let mut positionals = Vec::new();
    let mut skip_next = false;

    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }

        if keys.iter().any(|key| arg.eq_ignore_ascii_case(key)) {
            skip_next = true;
            continue;
        }

        if keys.iter().any(|key| parse_named_arg(arg, key).is_some()) {
            continue;
        }

        positionals.push(arg.clone());
    }

    positionals
}

fn print_usage(bad_command: Option<&str>) {
    if let Some(command) = bad_command {
        eprintln!("Comando no soportado: {command}");
    }
    eprintln!("Uso:");
    eprintln!("  l400cmd <CMD> [parámetros]");
    eprintln!("  <CMD> [parámetros]");
    eprintln!("Comandos disponibles:");
    for command in COMMAND_BINARIES {
        eprintln!("  {command}");
    }
    eprintln!("Notas:");
    eprintln!("  - Acepta FILE(QGPL/QCLSRC), FILE=QGPL/QCLSRC o argumentos posicionales.");
    eprintln!("  - STRSQL acepta stdin o una sentencia SQL entre comillas.");
}

#[cfg(test)]
mod tests {
    use super::{command_spec, parse_named_arg, positional_args};

    #[test]
    fn parse_named_arg_supports_parentheses() {
        assert_eq!(
            parse_named_arg("FILE(QGPL/QCLSRC)", "FILE"),
            Some("QGPL/QCLSRC".to_string())
        );
    }

    #[test]
    fn parse_named_arg_supports_equals() {
        assert_eq!(
            parse_named_arg("MBR=HELLO.CLP", "MBR"),
            Some("HELLO.CLP".to_string())
        );
    }

    #[test]
    fn positional_args_skip_named_assignments() {
        let args = vec![
            "FILE(QGPL/QCLSRC)".to_string(),
            "MBR".to_string(),
            "HELLO.CLP".to_string(),
            "EXTRA".to_string(),
        ];

        assert_eq!(positional_args(&args, &["FILE", "MBR"]), vec!["EXTRA"]);
    }

    #[test]
    fn command_spec_keeps_wrkobj_filters() {
        let args = vec![
            "OBJ(QSYS/WRK*)".to_string(),
            "OBJTYPE(*CMD)".to_string(),
            "LIB(QSYS)".to_string(),
        ];

        assert_eq!(
            command_spec(&args, &["OBJ", "OBJTYPE", "LIB"]),
            "OBJ=QSYS/WRK* OBJTYPE=*CMD LIB=QSYS"
        );
    }

    #[test]
    fn command_spec_maps_positional_object() {
        let args = vec!["QGPL/DEMO".to_string(), "CONFIRM(*YES)".to_string()];

        assert_eq!(
            command_spec(&args, &["OBJ", "CONFIRM"]),
            "CONFIRM=*YES OBJ=QGPL/DEMO"
        );
    }
}
