use crate::parser::parse_file;
use std::collections::BTreeSet;
use std::fs;

pub struct Compiler;

fn escape_c_string(input: &str) -> String {
    format!("{input:?}")
}

fn value_to_string(value: &crate::ast::Value) -> String {
    match value {
        crate::ast::Value::StringLiteral(value)
        | crate::ast::Value::Keyword(value)
        | crate::ast::Value::Identifier(value)
        | crate::ast::Value::Variable(value) => value.clone(),
        crate::ast::Value::List(values) => values
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn value_to_c_expr(value: &crate::ast::Value) -> String {
    match value {
        crate::ast::Value::Variable(value) => format!("var_{}", sanitize_c_identifier(value)),
        crate::ast::Value::List(values) => escape_c_string(
            &values
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => escape_c_string(&value_to_string(value)),
    }
}

fn value_is_numeric(value: &crate::ast::Value, numeric_variables: &BTreeSet<String>) -> bool {
    match value {
        crate::ast::Value::StringLiteral(value)
        | crate::ast::Value::Keyword(value)
        | crate::ast::Value::Identifier(value) => value.parse::<f64>().is_ok(),
        crate::ast::Value::Variable(value) => {
            numeric_variables.contains(&sanitize_c_identifier(value))
        }
        crate::ast::Value::List(_) => false,
    }
}

fn named_param<'a>(command: &'a crate::ast::Command, key: &str) -> Option<&'a crate::ast::Value> {
    for p in &command.parameters {
        if let crate::ast::Parameter::Named(k, v) = p
            && k.eq_ignore_ascii_case(key)
        {
            return Some(v);
        }
    }
    None
}

fn positional(command: &crate::ast::Command, index: usize) -> Option<String> {
    command
        .parameters
        .iter()
        .filter_map(|parameter| match parameter {
            crate::ast::Parameter::Positional(value) => Some(value_to_string(value)),
            crate::ast::Parameter::Named(_, _) => None,
        })
        .nth(index)
}

fn sanitize_c_identifier(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch.to_ascii_uppercase());
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "VAR".to_string()
    } else {
        output
    }
}

fn first_value(command: &crate::ast::Command, key: &str, fallback: &str) -> crate::ast::Value {
    named_param(command, key)
        .cloned()
        .or_else(|| {
            command
                .parameters
                .iter()
                .find_map(|parameter| match parameter {
                    crate::ast::Parameter::Positional(value) => Some(value.clone()),
                    crate::ast::Parameter::Named(_, _) => None,
                })
        })
        .unwrap_or_else(|| crate::ast::Value::Identifier(fallback.to_string()))
}

fn generate_assignment(variable: &str, value: &crate::ast::Value) -> String {
    format!(
        "snprintf(var_{}, sizeof(var_{}), \"%s\", {});",
        sanitize_c_identifier(variable),
        sanitize_c_identifier(variable),
        value_to_c_expr(value)
    )
}

fn command_parameter_to_spec(parameter: &crate::ast::Parameter) -> String {
    match parameter {
        crate::ast::Parameter::Positional(value) => value_to_string(value),
        crate::ast::Parameter::Named(key, value) => format!("{}({})", key, value_to_string(value)),
    }
}

fn command_parameter_to_equals_spec(parameter: &crate::ast::Parameter) -> String {
    match parameter {
        crate::ast::Parameter::Positional(value) => value_to_string(value),
        crate::ast::Parameter::Named(key, value) => format!("{}={}", key, value_to_string(value)),
    }
}

fn command_to_equals_spec(command: &crate::ast::Command) -> String {
    command
        .parameters
        .iter()
        .map(command_parameter_to_equals_spec)
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote_arg(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn command_to_shell_spec(command: &crate::ast::Command) -> String {
    command
        .parameters
        .iter()
        .map(command_parameter_to_spec)
        .map(|parameter| shell_quote_arg(&parameter))
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_command_call(command: &crate::ast::Command) -> String {
    match command.name.as_str() {
        "PGM" | "ENDPGM" => String::new(),
        "DCL" => {
            let Some(crate::ast::Value::Variable(var)) = named_param(command, "VAR") else {
                return "l400_sndpgmmsg(\"[clc] DCL requiere VAR(&NOMBRE)\");".to_string();
            };
            named_param(command, "VALUE")
                .map(|value| generate_assignment(var, value))
                .unwrap_or_else(|| {
                    if named_param(command, "TYPE")
                        .map(value_to_string)
                        .is_some_and(|value| value.contains("*DEC") || value.contains("*INT"))
                    {
                        generate_assignment(var, &crate::ast::Value::StringLiteral("0".to_string()))
                    } else {
                        format!("/* DCL &{} */", sanitize_c_identifier(var))
                    }
                })
        }
        "CHGVAR" => {
            let Some(crate::ast::Value::Variable(var)) = named_param(command, "VAR") else {
                return "l400_sndpgmmsg(\"[clc] CHGVAR requiere VAR(&NOMBRE)\");".to_string();
            };
            let value = named_param(command, "VALUE")
                .or_else(|| named_param(command, "VAL"))
                .cloned()
                .unwrap_or_else(|| crate::ast::Value::StringLiteral(String::new()));
            generate_assignment(var, &value)
        }

        "SNDPGMMSG" => {
            let msg = first_value(command, "MSG", "SNDPGMMSG sin mensaje");
            format!("l400_sndpgmmsg({});", value_to_c_expr(&msg))
        }

        // --- Gestión de sistema ---
        "WRKSYSSTS" => "l400_wrksyssts();".to_string(),
        "WRKACTJOB" => "l400_wrkactjob();".to_string(),
        "WRKSYSVAL" => "l400_wrksysval();".to_string(),
        "DSPLOG" => "l400_dsplog();".to_string(),
        "WRKSPLF" => "l400_wrksplf();".to_string(),
        "WRKOUTQ" => "l400_wrkoutq();".to_string(),
        "DSPSPLF" => {
            let spec = command_to_equals_spec(command);
            format!("l400_dspsplf({});", escape_c_string(&spec))
        }
        "CHGSPLFA" => {
            let spec = command_to_equals_spec(command);
            format!("l400_chgsplfa({});", escape_c_string(&spec))
        }
        "DLTSPLF" => {
            let spec = command_to_equals_spec(command);
            format!("l400_dltsplf({});", escape_c_string(&spec))
        }
        "SBMJOB" => {
            let spec = format!("l400cmd SBMJOB {}", command_to_shell_spec(command));
            format!("system({});", escape_c_string(&spec))
        }
        "WRKUSRPRF" => {
            let profile = first_value(command, "USRPRF", "*ALL");
            format!("l400_wrkusrprf({});", value_to_c_expr(&profile))
        }
        "PWRDWNSYS" => {
            let opt = first_value(command, "OPTION", "*CNTRLD");
            format!("l400_pwrdwnsys({});", value_to_c_expr(&opt))
        }

        // --- Objetos y bibliotecas ---
        "WRKOBJ" => {
            let obj = first_value(command, "OBJ", "*ALL");
            format!("l400_wrkobj({});", value_to_c_expr(&obj))
        }
        "CRTLIB" => {
            let lib = first_value(command, "LIB", "NEWLIB");
            format!("l400_crtlib({});", value_to_c_expr(&lib))
        }
        "DLTLIB" => {
            let lib = first_value(command, "LIB", "NEWLIB");
            format!("l400_dltlib({});", value_to_c_expr(&lib))
        }
        "ADDLIBLE" => {
            let lib = first_value(command, "LIB", "QGPL");
            format!("l400_addlible({});", value_to_c_expr(&lib))
        }
        "CHGCURLIB" => {
            let lib = first_value(command, "CURLIB", "QGPL");
            format!("l400_chgcurlib({});", value_to_c_expr(&lib))
        }
        "RNMOBJ" => {
            let obj = first_value(command, "OBJ", "");
            let newname = first_value(command, "NEWOBJ", "RENAMED");
            format!(
                "l400_rnmobj({}, {});",
                value_to_c_expr(&obj),
                value_to_c_expr(&newname)
            )
        }

        // --- Programación ---
        "CRTPGM" => {
            let pgm = first_value(command, "PGM", "");
            format!("l400_crtpgm({});", value_to_c_expr(&pgm))
        }
        "CRTCLPGM" => {
            let pgm = first_value(command, "PGM", "");
            let srcfile = first_value(command, "SRCFILE", "QGPL/QCLSRC");
            let srcmbr = first_value(command, "SRCMBR", "MAIN");
            format!(
                "l400_crtclpgm({}, {}, {});",
                value_to_c_expr(&pgm),
                value_to_c_expr(&srcfile),
                value_to_c_expr(&srcmbr)
            )
        }
        "CALL" => {
            let pgm = first_value(command, "PGM", "");
            format!("l400_call({});", value_to_c_expr(&pgm))
        }

        // --- Navegación / sesión ---
        "GO" => {
            let target = first_value(command, "MENU", "MAIN");
            format!("l400_go({});", value_to_c_expr(&target))
        }
        "SIGNOFF" => "l400_signoff();".to_string(),

        "STRPDM" => "l400_strpdm();".to_string(),
        "STRSEU" => {
            let file = named_param(command, "FILE")
                .map(value_to_string)
                .or_else(|| positional(command, 0))
                .unwrap_or_else(|| "QGPL/QCLSRC".to_string());
            let member = named_param(command, "MBR")
                .map(value_to_string)
                .or_else(|| positional(command, 1))
                .unwrap_or_else(|| "MAIN.CLP".to_string());
            format!(
                "l400_strseu({}, {});",
                escape_c_string(&file),
                escape_c_string(&member)
            )
        }
        "STRSQL" => "l400_strsql();".to_string(),
        "WRKMBRPDM" => {
            let file = named_param(command, "FILE")
                .map(value_to_string)
                .or_else(|| positional(command, 0))
                .unwrap_or_else(|| "QGPL/QCLSRC".to_string());
            format!("l400_wrkmbrpdm({});", escape_c_string(&file))
        }

        other => format!(
            "l400_sndpgmmsg({});",
            escape_c_string(&format!("[clc] Comando CL no soportado en v2: {other}"))
        ),
    }
}

fn msgid_to_numeric(msgid: &str) -> u32 {
    msgid
        .trim()
        .strip_prefix("CPF")
        .unwrap_or(msgid.trim())
        .parse::<u32>()
        .unwrap_or(0)
}

fn condition_to_c(
    condition: &crate::ast::Condition,
    numeric_variables: &BTreeSet<String>,
) -> String {
    let left = value_to_c_expr(&condition.left);
    let right = value_to_c_expr(&condition.right);
    let numeric_compare = value_is_numeric(&condition.left, numeric_variables)
        && value_is_numeric(&condition.right, numeric_variables);
    match condition.operator.as_str() {
        "*EQ" | "=" | "EQ" => format!("strcmp({left}, {right}) == 0"),
        "*NE" | "<>" | "NE" => format!("strcmp({left}, {right}) != 0"),
        "*GT" | ">" | "GT" if numeric_compare => {
            format!("atof({left}) > atof({right})")
        }
        "*LT" | "<" | "LT" if numeric_compare => {
            format!("atof({left}) < atof({right})")
        }
        "*GE" | ">=" | "GE" if numeric_compare => {
            format!("atof({left}) >= atof({right})")
        }
        "*LE" | "<=" | "LE" if numeric_compare => {
            format!("atof({left}) <= atof({right})")
        }
        "*GT" | ">" | "GT" => format!("strcmp({left}, {right}) > 0"),
        "*LT" | "<" | "LT" => format!("strcmp({left}, {right}) < 0"),
        "*GE" | ">=" | "GE" => format!("strcmp({left}, {right}) >= 0"),
        "*LE" | "<=" | "LE" => format!("strcmp({left}, {right}) <= 0"),
        _ => "0".to_string(),
    }
}

fn generate_statement(
    statement: &crate::ast::Statement,
    indent: usize,
    out: &mut Vec<String>,
    numeric_variables: &BTreeSet<String>,
) {
    let pad = "    ".repeat(indent);
    match statement {
        crate::ast::Statement::Command(command) => {
            let line = generate_command_call(command);
            if !line.is_empty() {
                out.push(format!("{pad}l400_clear_status();"));
                out.push(format!("{pad}{line}"));
            }
        }
        crate::ast::Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            out.push(format!(
                "{pad}if ({}) {{",
                condition_to_c(condition, numeric_variables)
            ));
            for statement in then_branch {
                generate_statement(statement, indent + 1, out, numeric_variables);
            }
            if else_branch.is_empty() {
                out.push(format!("{pad}}}"));
            } else {
                out.push(format!("{pad}}} else {{"));
                for statement in else_branch {
                    generate_statement(statement, indent + 1, out, numeric_variables);
                }
                out.push(format!("{pad}}}"));
            }
        }
        crate::ast::Statement::MonMsg { msgid, exec } => {
            let code = msgid_to_numeric(msgid);
            if let Some(exec) = exec {
                let line = generate_command_call(exec);
                if !line.is_empty() {
                    if code == 0 {
                        out.push(format!("{pad}if (l400_last_cpf_code() != 0) {{"));
                    } else {
                        out.push(format!("{pad}if (l400_last_cpf_code() == {code}) {{"));
                    }
                    out.push(format!("{pad}    l400_clear_status();"));
                    out.push(format!("{pad}    {line}"));
                    out.push(format!("{pad}}}"));
                }
            }
        }
        crate::ast::Statement::While {
            condition,
            body,
            until,
        } => {
            let condition = condition_to_c(condition, numeric_variables);
            let test = if *until {
                format!("!({condition})")
            } else {
                condition
            };
            out.push(format!("{pad}while ({test}) {{"));
            for statement in body {
                generate_statement(statement, indent + 1, out, numeric_variables);
            }
            out.push(format!("{pad}}}"));
        }
    }
}

fn collect_declared_variables(ast: &crate::ast::Program) -> BTreeSet<String> {
    let mut vars = ast
        .parameters
        .iter()
        .map(|value| sanitize_c_identifier(value))
        .collect::<BTreeSet<_>>();
    for command in &ast.commands {
        if command.name == "DCL"
            && let Some(crate::ast::Value::Variable(var)) = named_param(command, "VAR")
        {
            vars.insert(sanitize_c_identifier(var));
        }
    }
    vars
}

fn collect_numeric_variables(ast: &crate::ast::Program) -> BTreeSet<String> {
    let mut vars = BTreeSet::new();
    for command in &ast.commands {
        collect_numeric_variable_from_command(command, &mut vars);
    }
    for statement in &ast.statements {
        collect_numeric_variables_from_statement(statement, &mut vars);
    }
    vars
}

fn collect_numeric_variable_from_command(
    command: &crate::ast::Command,
    vars: &mut BTreeSet<String>,
) {
    if command.name == "DCL" {
        let is_numeric = named_param(command, "TYPE")
            .map(value_to_string)
            .is_some_and(|value| value.contains("*DEC") || value.contains("*INT"));
        if is_numeric && let Some(crate::ast::Value::Variable(var)) = named_param(command, "VAR") {
            vars.insert(sanitize_c_identifier(var));
        }
    }
}

fn collect_numeric_variables_from_statement(
    statement: &crate::ast::Statement,
    vars: &mut BTreeSet<String>,
) {
    match statement {
        crate::ast::Statement::Command(command) => {
            collect_numeric_variable_from_command(command, vars);
        }
        crate::ast::Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            for statement in then_branch.iter().chain(else_branch) {
                collect_numeric_variables_from_statement(statement, vars);
            }
        }
        crate::ast::Statement::MonMsg { exec, .. } => {
            if let Some(command) = exec {
                collect_numeric_variable_from_command(command, vars);
            }
        }
        crate::ast::Statement::While { body, .. } => {
            for statement in body {
                collect_numeric_variables_from_statement(statement, vars);
            }
        }
    }
}

fn generate_c_backend(source_path: &str, ast: &crate::ast::Program) -> String {
    let mut body = Vec::new();
    body.push(format!(
        "l400_sndpgmmsg(\"[clc] Ejecutando programa CL compilado desde {}\");",
        source_path.replace('"', "\\\"")
    ));

    let declared_variables = collect_declared_variables(ast);
    let numeric_variables = collect_numeric_variables(ast);
    for (index, parameter) in ast.parameters.iter().enumerate() {
        let var = sanitize_c_identifier(parameter);
        body.push(format!(
            "if (argc > {}) snprintf(var_{}, sizeof(var_{}), \"%s\", argv[{}]);",
            index + 1,
            var,
            var,
            index + 1
        ));
    }

    for statement in &ast.statements {
        generate_statement(statement, 1, &mut body, &numeric_variables);
    }

    let declarations = declared_variables
        .iter()
        .map(|var| format!("    char var_{var}[1024] = \"\";"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "#include <stdio.h>\n\
         #include <string.h>\n\
         #include <stdlib.h>\n\
         extern void l400_sndpgmmsg(const char*);\n\
         extern unsigned int l400_last_cpf_code(void);\n\
         extern void l400_clear_status(void);\n\
         extern void l400_wrksyssts(void);\n\
         extern void l400_wrkactjob(void);\n\
         extern void l400_wrksysval(void);\n\
         extern void l400_dsplog(void);\n\
         extern void l400_wrksplf(void);\n\
         extern void l400_wrkoutq(void);\n\
         extern void l400_dspsplf(const char*);\n\
         extern void l400_chgsplfa(const char*);\n\
         extern void l400_dltsplf(const char*);\n\
         extern void l400_wrkusrprf(const char*);\n\
         extern void l400_pwrdwnsys(const char*);\n\
         extern void l400_wrkobj(const char*);\n\
         extern void l400_crtlib(const char*);\n\
         extern void l400_dltlib(const char*);\n\
         extern void l400_addlible(const char*);\n\
         extern void l400_chgcurlib(const char*);\n\
         extern void l400_rnmobj(const char*, const char*);\n\
         extern void l400_crtpgm(const char*);\n\
         extern void l400_crtclpgm(const char*, const char*, const char*);\n\
         extern void l400_call(const char*);\n\
         extern void l400_go(const char*);\n\
         extern void l400_signoff(void);\n\
         extern void l400_strpdm(void);\n\
         extern void l400_strseu(const char*, const char*);\n\
         extern void l400_strsql(void);\n\
         extern void l400_wrkmbrpdm(const char*);\n\
         \n\
         int main(int argc, char** argv) {{\n{}\n    {}\n    return 0;\n}}\n",
        declarations,
        body.join("\n    ")
    )
}

impl Compiler {
    pub fn compile(source_path: &str, output_path: &str) -> Result<(), String> {
        // 1. Leer fuente CL
        let source_code = fs::read_to_string(source_path)
            .map_err(|e| format!("Error leyendo el archivo fuente: {}", e))?;

        // 2. Parsear código CL (Pest -> AST)
        let ast =
            parse_file(&source_code).map_err(|e| format!("Error de Análisis Sintáctico: {}", e))?;

        println!(
            "AST procesado exitosamente: {} comandos",
            ast.commands.len()
        );

        // 3. Generar código objeto
        #[cfg(feature = "llvm-backend")]
        {
            use crate::codegen::CodeGenerator;
            use inkwell::context::Context;
            let context = Context::create();
            let codegen = CodeGenerator::new(&context, "cl_module");
            codegen.generate_program(&ast)?;
            codegen.emit_object_file(output_path)?;
        }

        #[cfg(not(feature = "llvm-backend"))]
        {
            // Sin backend LLVM: emitir código C nativo
            println!(">> Emitiendo código nativo vía backend C.");
            use std::io::Write;
            let c_code = generate_c_backend(source_path, &ast);
            let c_file = format!("{}.tmp.c", output_path);
            let mut f = fs::File::create(&c_file)
                .map_err(|e| format!("Error creando archivo C temporal: {}", e))?;
            f.write_all(c_code.as_bytes())
                .map_err(|e| format!("Error escribiendo archivo C: {}", e))?;

            let c_compiler = if std::process::Command::new("clang")
                .arg("--version")
                .output()
                .is_ok()
            {
                "clang"
            } else {
                "cc"
            };

            let status = std::process::Command::new(c_compiler)
                .arg(&c_file)
                .arg("-c")
                .arg("-o")
                .arg(output_path)
                .status()
                .map_err(|e| format!("Error ejecutando {}: {}", c_compiler, e))?;

            let _ = fs::remove_file(&c_file);

            if !status.success() {
                return Err(format!("{c_compiler} falló al compilar el backend C"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Command, Condition, Parameter, Program, Statement, Value};

    fn program_from_commands(commands: Vec<Command>) -> Program {
        Program {
            statements: commands.iter().cloned().map(Statement::Command).collect(),
            commands,
            parameters: Vec::new(),
        }
    }

    #[test]
    fn generate_c_backend_emits_sndpgmmsg_output() {
        let program = program_from_commands(vec![
            Command {
                name: "PGM".to_string(),
                parameters: vec![],
            },
            Command {
                name: "SNDPGMMSG".to_string(),
                parameters: vec![Parameter::Positional(Value::StringLiteral(
                    "Hola desde CL".to_string(),
                ))],
            },
            Command {
                name: "ENDPGM".to_string(),
                parameters: vec![],
            },
        ]);

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("Hola desde CL"));
        assert!(code.contains("Ejecutando programa CL"));
        assert!(code.contains("l400_sndpgmmsg"));
    }

    #[test]
    fn generate_c_backend_marks_unsupported_commands() {
        let program = program_from_commands(vec![Command {
            name: "DLTOBJ".to_string(),
            parameters: vec![],
        }]);

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("no soportado en v2: DLTOBJ"));
    }

    #[test]
    fn generate_crtlib_call() {
        let program = program_from_commands(vec![Command {
            name: "CRTLIB".to_string(),
            parameters: vec![Parameter::Named(
                "LIB".to_string(),
                Value::Identifier("MYLIB".to_string()),
            )],
        }]);
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_crtlib"));
        assert!(code.contains("MYLIB"));
    }

    #[test]
    fn generate_wrkactjob_call() {
        let program = program_from_commands(vec![Command {
            name: "WRKACTJOB".to_string(),
            parameters: vec![],
        }]);
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_wrkactjob()"));
    }

    #[test]
    fn generate_signoff_call() {
        let program = program_from_commands(vec![Command {
            name: "SIGNOFF".to_string(),
            parameters: vec![],
        }]);
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_signoff()"));
    }

    #[test]
    fn interactive_commands_emit_real_calls() {
        let program = program_from_commands(vec![
            Command {
                name: "STRPDM".to_string(),
                parameters: vec![],
            },
            Command {
                name: "STRSEU".to_string(),
                parameters: vec![
                    Parameter::Named(
                        "FILE".to_string(),
                        Value::Identifier("QGPL/QCLSRC".to_string()),
                    ),
                    Parameter::Named(
                        "MBR".to_string(),
                        Value::Identifier("HELLO.CLP".to_string()),
                    ),
                ],
            },
            Command {
                name: "STRSQL".to_string(),
                parameters: vec![],
            },
            Command {
                name: "WRKMBRPDM".to_string(),
                parameters: vec![Parameter::Named(
                    "FILE".to_string(),
                    Value::Identifier("QGPL/QCLSRC".to_string()),
                )],
            },
        ]);
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_strpdm();"));
        assert!(code.contains("l400_strseu(\"QGPL/QCLSRC\", \"HELLO.CLP\");"));
        assert!(code.contains("l400_strsql();"));
        assert!(code.contains("l400_wrkmbrpdm(\"QGPL/QCLSRC\");"));
    }

    #[test]
    fn monmsg_checks_runtime_cpf_status() {
        let program = Program {
            commands: Vec::new(),
            statements: vec![
                Statement::Command(Command {
                    name: "CALL".to_string(),
                    parameters: vec![Parameter::Named(
                        "PGM".to_string(),
                        Value::Identifier("QGPL/MISSING".to_string()),
                    )],
                }),
                Statement::MonMsg {
                    msgid: "CPF2204".to_string(),
                    exec: Some(Command {
                        name: "SNDPGMMSG".to_string(),
                        parameters: vec![Parameter::Named(
                            "MSG".to_string(),
                            Value::StringLiteral("caught".to_string()),
                        )],
                    }),
                },
            ],
            parameters: Vec::new(),
        };
        let code = generate_c_backend("demo.clp", &program);

        assert!(code.contains("l400_last_cpf_code() == 2204"));
        assert!(code.contains("l400_clear_status();"));
        assert!(code.contains("caught"));
    }

    #[test]
    fn control_flow_variables_and_toolchain_emit_c() {
        let program = Program {
            commands: Vec::new(),
            parameters: vec!["TARGET".to_string()],
            statements: vec![
                Statement::Command(Command {
                    name: "DCL".to_string(),
                    parameters: vec![
                        Parameter::Named("VAR".to_string(), Value::Variable("TARGET".to_string())),
                        Parameter::Named(
                            "VALUE".to_string(),
                            Value::StringLiteral("DEMO".to_string()),
                        ),
                    ],
                }),
                Statement::If {
                    condition: Condition {
                        left: Value::Variable("TARGET".to_string()),
                        operator: "*EQ".to_string(),
                        right: Value::StringLiteral("DEMO".to_string()),
                    },
                    then_branch: vec![Statement::Command(Command {
                        name: "CALL".to_string(),
                        parameters: vec![Parameter::Named(
                            "PGM".to_string(),
                            Value::Identifier("QGPL/HELLO".to_string()),
                        )],
                    })],
                    else_branch: vec![Statement::Command(Command {
                        name: "CRTCLPGM".to_string(),
                        parameters: vec![
                            Parameter::Named(
                                "PGM".to_string(),
                                Value::Identifier("QGPL/HELLO".to_string()),
                            ),
                            Parameter::Named(
                                "SRCFILE".to_string(),
                                Value::Identifier("QGPL/QCLSRC".to_string()),
                            ),
                            Parameter::Named(
                                "SRCMBR".to_string(),
                                Value::Identifier("HELLO.CLP".to_string()),
                            ),
                        ],
                    })],
                },
            ],
        };

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("char var_TARGET"));
        assert!(code.contains("strcmp(var_TARGET, \"DEMO\") == 0"));
        assert!(code.contains("l400_call(\"QGPL/HELLO\");"));
        assert!(code.contains("l400_crtclpgm(\"QGPL/HELLO\", \"QGPL/QCLSRC\", \"HELLO.CLP\");"));
    }

    #[test]
    fn numeric_loops_and_spool_commands_emit_c() {
        let program = Program {
            commands: Vec::new(),
            parameters: Vec::new(),
            statements: vec![
                Statement::Command(Command {
                    name: "DCL".to_string(),
                    parameters: vec![
                        Parameter::Named("VAR".to_string(), Value::Variable("COUNT".to_string())),
                        Parameter::Named("TYPE".to_string(), Value::Keyword("*DEC".to_string())),
                    ],
                }),
                Statement::While {
                    condition: Condition {
                        left: Value::Variable("COUNT".to_string()),
                        operator: "*LT".to_string(),
                        right: Value::Identifier("3".to_string()),
                    },
                    body: vec![Statement::Command(Command {
                        name: "SNDPGMMSG".to_string(),
                        parameters: vec![Parameter::Named(
                            "MSG".to_string(),
                            Value::StringLiteral("loop".to_string()),
                        )],
                    })],
                    until: false,
                },
                Statement::Command(Command {
                    name: "WRKSPLF".to_string(),
                    parameters: vec![],
                }),
                Statement::Command(Command {
                    name: "DSPSPLF".to_string(),
                    parameters: vec![Parameter::Named(
                        "FILE".to_string(),
                        Value::Identifier("LAST".to_string()),
                    )],
                }),
                Statement::Command(Command {
                    name: "SBMJOB".to_string(),
                    parameters: vec![Parameter::Named(
                        "CMD".to_string(),
                        Value::Identifier("WRKSYSSTS".to_string()),
                    )],
                }),
            ],
        };

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("snprintf(var_COUNT"));
        assert!(code.contains("while (atof(var_COUNT) < atof(\"3\"))"));
        assert!(code.contains("l400_wrksplf();"));
        assert!(code.contains("l400_dspsplf(\"FILE=LAST\");"));
        assert!(code.contains("system(\"l400cmd SBMJOB 'CMD(WRKSYSSTS)'\");"));
    }

    #[test]
    fn sbmjob_specs_are_quoted_for_shell_system_calls() {
        let program = Program {
            commands: Vec::new(),
            parameters: Vec::new(),
            statements: vec![Statement::Command(Command {
                name: "SBMJOB".to_string(),
                parameters: vec![Parameter::Named(
                    "CMD".to_string(),
                    Value::StringLiteral("CALL PGM(HELLO)".to_string()),
                )],
            })],
        };

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("system(\"l400cmd SBMJOB 'CMD(CALL PGM(HELLO))'\");"));
    }

    #[test]
    fn character_variable_relational_conditions_use_string_compare() {
        let program = Program {
            commands: Vec::new(),
            parameters: Vec::new(),
            statements: vec![
                Statement::Command(Command {
                    name: "DCL".to_string(),
                    parameters: vec![
                        Parameter::Named("VAR".to_string(), Value::Variable("STATUS".to_string())),
                        Parameter::Named("TYPE".to_string(), Value::Keyword("*CHAR".to_string())),
                    ],
                }),
                Statement::If {
                    condition: Condition {
                        left: Value::Variable("STATUS".to_string()),
                        operator: "*GT".to_string(),
                        right: Value::StringLiteral("A".to_string()),
                    },
                    then_branch: vec![Statement::Command(Command {
                        name: "SNDPGMMSG".to_string(),
                        parameters: vec![Parameter::Named(
                            "MSG".to_string(),
                            Value::StringLiteral("text".to_string()),
                        )],
                    })],
                    else_branch: Vec::new(),
                },
            ],
        };

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("strcmp(var_STATUS, \"A\") > 0"));
        assert!(!code.contains("atof(var_STATUS)"));
    }
}
