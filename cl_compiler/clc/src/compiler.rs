use crate::parser::parse_file;
use std::fs;

pub struct Compiler;

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
            // Sin backend LLVM: emitir un stub de ELF vía shell-out a clang con IR vacío
            println!("[WARN] Backend LLVM no habilitado. Emitiendo objeto stub.");
            use std::io::Write;
            let c_stub = format!(
                "// Generado por clc (modo stub)\n// Fuente CL: {}\n\nvoid cl_main() {{}}\nint main() {{ cl_main(); return 0; }}\n",
                source_path
            );
            let stub_c = format!("{}.stub.c", output_path);
            let mut f =
                fs::File::create(&stub_c).map_err(|e| format!("Error creando stub C: {}", e))?;
            f.write_all(c_stub.as_bytes())
                .map_err(|e| format!("Error escribiendo stub: {}", e))?;

            let status = std::process::Command::new("clang")
                .arg(&stub_c)
                .arg("-c")
                .arg("-o")
                .arg(output_path)
                .status()
                .map_err(|e| format!("Error ejecutando clang: {}", e))?;

            let _ = fs::remove_file(&stub_c);

            if !status.success() {
                return Err("clang falló al compilar el stub".to_string());
            }
        }

        Ok(())
    }
}
