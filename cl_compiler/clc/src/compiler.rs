use std::fs;
use crate::parser::parse_file;
use crate::codegen::CodeGenerator;
use inkwell::context::Context;

pub struct Compiler;

impl Compiler {
    pub fn compile(source_path: &str, output_path: &str) -> Result<(), String> {
        // 1. Leer fuenta CL
        let source_code = fs::read_to_string(source_path)
            .map_err(|e| format!("Error leyendo el archivo fuente: {}", e))?;

        // 2. Parsar código CL (Pest -> AST)
        let ast = parse_file(&source_code)
            .map_err(|e| format!("Error de Análisis Sintáctico: {}", e))?;

        println!("AST procesado exitosamente: {} comandos", ast.commands.len());

        // 3. Generar IR de LLVM embebido y escribir código base (.o)
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "cl_module");
        
        codegen.generate_program(&ast)?;
        codegen.emit_object_file(output_path)?;

        Ok(())
    }
}
