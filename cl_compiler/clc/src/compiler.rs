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

fn extract_sndpgmmsg(command: &crate::ast::Command) -> Option<String> {
    for parameter in &command.parameters {
        match parameter {
            crate::ast::Parameter::Named(name, value) if name == "MSG" => {
                return Some(value_to_string(value));
            }
            crate::ast::Parameter::Positional(value) => {
                return Some(value_to_string(value));
            }
            _ => {}
        }
    }

    None
}

fn generate_c_backend(source_path: &str, ast: &crate::ast::Program) -> String {
    let mut body = Vec::new();
    body.push(format!(
        "l400_sndpgmmsg(\"[clc] Executing CL program compiled from {}\");",
        source_path.replace('"', "\\\"")
    ));

    for command in &ast.commands {
        match command.name.as_str() {
            "PGM" | "ENDPGM" => {}
            "SNDPGMMSG" => {
                let message = extract_sndpgmmsg(command)
                    .unwrap_or_else(|| "SNDPGMMSG without message".to_string());
                body.push(format!("l400_sndpgmmsg({});", escape_c_string(&message)));
            }
            other => body.push(format!(
                "l400_sndpgmmsg({});",
                escape_c_string(&format!(
                    "[clc] Unsupported CL command in v1 subset: {other}"
                ))
            )),
        }
    }

    format!(
        "#include <stdio.h>\nextern void l400_sndpgmmsg(const char*);\n\nint main(void) {{\n    {}\n    return 0;\n}}\n",
        body.join("\n    ")
    )
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
            let mut f =
                fs::File::create(&c_file).map_err(|e| format!("Error creando archivo C temporal: {}", e))?;
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
        assert!(code.contains("Executing CL program"));
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
        assert!(code.contains("Unsupported CL command in v1 subset: DLTOBJ"));
    }
}
