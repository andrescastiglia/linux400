use std::env;
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::Path;
use std::process::ExitCode;

use l400::ffi_commands;

const COMMAND_BINARIES: &[&str] = &[
    "WRKSYSSTS",
    "WRKACTJOB",
    "WRKSYSVAL",
    "DSPLOG",
    "WRKUSRPRF",
    "PWRDWNSYS",
    "WRKOBJ",
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
        "WRKACTJOB" => {
            ffi_commands::l400_wrkactjob();
            ExitCode::SUCCESS
        }
        "WRKSYSVAL" => {
            ffi_commands::l400_wrksysval();
            ExitCode::SUCCESS
        }
        "DSPLOG" => {
            ffi_commands::l400_dsplog();
            ExitCode::SUCCESS
        }
        "WRKUSRPRF" => dispatch_unary(args, &["USRPRF"], None, ffi_commands::l400_wrkusrprf),
        "PWRDWNSYS" => dispatch_unary(args, &["OPTION"], None, ffi_commands::l400_pwrdwnsys),
        "WRKOBJ" => dispatch_unary(
            args,
            &["OBJ", "FILTER", "OBJTYPE"],
            None,
            ffi_commands::l400_wrkobj,
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
            &["LIB"],
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
        "GO" => dispatch_unary_required(command, args, &["MENU"], "GO MAIN", ffi_commands::l400_go),
        "SIGNOFF" => {
            ffi_commands::l400_signoff();
            ExitCode::SUCCESS
        }
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
        _ => {
            eprintln!("ERROR: comando Linux/400 no reconocido: {command}");
            print_usage(Some(command));
            ExitCode::from(2)
        }
    }
}

fn dispatch_unary(
    args: &[String],
    keys: &[&str],
    default: Option<&str>,
    callback: extern "C" fn(*const c_char),
) -> ExitCode {
    let value =
        extract_named_arg(args, keys).or_else(|| positional_args(args, keys).first().cloned());
    match value.or_else(|| default.map(str::to_string)) {
        Some(value) => call_with_cstring(&value, callback),
        None => {
            callback(std::ptr::null());
            ExitCode::SUCCESS
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
    match l400::run_select_query(&statement, None) {
        Ok(result) => {
            println!("{}", result.columns.join(" | "));
            if result.rows.is_empty() {
                println!("(sin filas)");
            } else {
                for row in result.rows {
                    println!("{}", row.join(" | "));
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("[STRSQL] Error: {error}");
            ExitCode::from(1)
        }
    }
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
    use super::{parse_named_arg, positional_args};

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
}
