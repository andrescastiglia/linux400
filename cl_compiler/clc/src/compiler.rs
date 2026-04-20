use crate::parser::parse_file;
use std::fs;

pub struct Compiler;

fn escape_c_string(input: &str) -> String {
    format!("{input:?}")
}

fn value_to_string(value: &crate::ast::Value) -> String {
    match value {
        crate::ast::Value::StringLiteral(value)
        | crate::ast::Value::Keyword(value)
        | crate::ast::Value::Identifier(value) => value.clone(),
    }
}

fn named_param<'a>(command: &'a crate::ast::Command, key: &str) -> Option<&'a crate::ast::Value> {
    for p in &command.parameters {
        if let crate::ast::Parameter::Named(k, v) = p {
            if k.eq_ignore_ascii_case(key) {
                return Some(v);
            }
        }
    }
    None
}

fn first_positional(command: &crate::ast::Command) -> Option<String> {
    for p in &command.parameters {
        if let crate::ast::Parameter::Positional(v) = p {
            return Some(value_to_string(v));
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

fn generate_command_call(command: &crate::ast::Command) -> String {
    match command.name.as_str() {
        "PGM" | "ENDPGM" => String::new(),

        "SNDPGMMSG" => {
            let msg = named_param(command, "MSG")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "SNDPGMMSG sin mensaje".to_string());
            format!("l400_sndpgmmsg({});", escape_c_string(&msg))
        }

        // --- Gestión de sistema ---
        "WRKSYSSTS" => "l400_wrksyssts();".to_string(),
        "WRKACTJOB" => "l400_wrkactjob();".to_string(),
        "WRKSYSVAL" => "l400_wrksysval();".to_string(),
        "DSPLOG" => "l400_dsplog();".to_string(),
        "WRKUSRPRF" => {
            let profile = named_param(command, "USRPRF")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "*ALL".to_string());
            format!("l400_wrkusrprf({});", escape_c_string(&profile))
        }
        "PWRDWNSYS" => {
            let opt = named_param(command, "OPTION")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "*CNTRLD".to_string());
            format!("l400_pwrdwnsys({});", escape_c_string(&opt))
        }

        // --- Objetos y bibliotecas ---
        "WRKOBJ" => {
            let obj = named_param(command, "OBJ")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "*ALL".to_string());
            format!("l400_wrkobj({});", escape_c_string(&obj))
        }
        "CRTLIB" => {
            let lib = named_param(command, "LIB")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "NEWLIB".to_string());
            format!("l400_crtlib({});", escape_c_string(&lib))
        }
        "DLTLIB" => {
            let lib = named_param(command, "LIB")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "NEWLIB".to_string());
            format!("l400_dltlib({});", escape_c_string(&lib))
        }
        "ADDLIBLE" => {
            let lib = named_param(command, "LIB")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "QGPL".to_string());
            format!("l400_addlible({});", escape_c_string(&lib))
        }
        "CHGCURLIB" => {
            let lib = named_param(command, "CURLIB")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_else(|| "QGPL".to_string());
            format!("l400_chgcurlib({});", escape_c_string(&lib))
        }
        "RNMOBJ" => {
            let obj = named_param(command, "OBJ")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_default();
            let newname = named_param(command, "NEWOBJ")
                .map(value_to_string)
                .unwrap_or_else(|| "RENAMED".to_string());
            format!(
                "l400_rnmobj({}, {});",
                escape_c_string(&obj),
                escape_c_string(&newname)
            )
        }

        // --- Programación ---
        "CRTPGM" => {
            let pgm = named_param(command, "PGM")
                .map(value_to_string)
                .or_else(|| first_positional(command))
                .unwrap_or_default();
            format!("l400_crtpgm({});", escape_c_string(&pgm))
        }

        // --- Navegación / sesión ---
        "GO" => {
            let target = first_positional(command).unwrap_or_else(|| "MAIN".to_string());
            format!("l400_go({});", escape_c_string(&target))
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

fn generate_c_backend(source_path: &str, ast: &crate::ast::Program) -> String {
    let mut body = Vec::new();
    body.push(format!(
        "l400_sndpgmmsg(\"[clc] Ejecutando programa CL compilado desde {}\");",
        source_path.replace('"', "\\\"")
    ));

    for command in &ast.commands {
        let line = generate_command_call(command);
        if !line.is_empty() {
            body.push(line);
        }
    }

    format!(
        "#include <stdio.h>\n\
         extern void l400_sndpgmmsg(const char*);\n\
         extern void l400_wrksyssts(void);\n\
         extern void l400_wrkactjob(void);\n\
         extern void l400_wrksysval(void);\n\
         extern void l400_dsplog(void);\n\
         extern void l400_wrkusrprf(const char*);\n\
         extern void l400_pwrdwnsys(const char*);\n\
         extern void l400_wrkobj(const char*);\n\
         extern void l400_crtlib(const char*);\n\
         extern void l400_dltlib(const char*);\n\
         extern void l400_addlible(const char*);\n\
         extern void l400_chgcurlib(const char*);\n\
         extern void l400_rnmobj(const char*, const char*);\n\
         extern void l400_crtpgm(const char*);\n\
         extern void l400_go(const char*);\n\
         extern void l400_signoff(void);\n\
         extern void l400_strpdm(void);\n\
         extern void l400_strseu(const char*, const char*);\n\
         extern void l400_strsql(void);\n\
         extern void l400_wrkmbrpdm(const char*);\n\
         \n\
         int main(void) {{\n    {}\n    return 0;\n}}\n",
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
    use crate::ast::{Command, Parameter, Program, Value};

    #[test]
    fn generate_c_backend_emits_sndpgmmsg_output() {
        let program = Program {
            commands: vec![
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
            ],
        };

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("Hola desde CL"));
        assert!(code.contains("Ejecutando programa CL"));
        assert!(code.contains("l400_sndpgmmsg"));
    }

    #[test]
    fn generate_c_backend_marks_unsupported_commands() {
        let program = Program {
            commands: vec![Command {
                name: "DLTOBJ".to_string(),
                parameters: vec![],
            }],
        };

        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("no soportado en v2: DLTOBJ"));
    }

    #[test]
    fn generate_crtlib_call() {
        let program = Program {
            commands: vec![Command {
                name: "CRTLIB".to_string(),
                parameters: vec![Parameter::Named(
                    "LIB".to_string(),
                    Value::Identifier("MYLIB".to_string()),
                )],
            }],
        };
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_crtlib"));
        assert!(code.contains("MYLIB"));
    }

    #[test]
    fn generate_wrkactjob_call() {
        let program = Program {
            commands: vec![Command {
                name: "WRKACTJOB".to_string(),
                parameters: vec![],
            }],
        };
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_wrkactjob()"));
    }

    #[test]
    fn generate_signoff_call() {
        let program = Program {
            commands: vec![Command {
                name: "SIGNOFF".to_string(),
                parameters: vec![],
            }],
        };
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_signoff()"));
    }

    #[test]
    fn interactive_commands_emit_real_calls() {
        let program = Program {
            commands: vec![
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
            ],
        };
        let code = generate_c_backend("demo.clp", &program);
        assert!(code.contains("l400_strpdm();"));
        assert!(code.contains("l400_strseu(\"QGPL/QCLSRC\", \"HELLO.CLP\");"));
        assert!(code.contains("l400_strsql();"));
        assert!(code.contains("l400_wrkmbrpdm(\"QGPL/QCLSRC\");"));
    }
}
