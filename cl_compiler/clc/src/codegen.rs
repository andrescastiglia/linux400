// Codegen de CL utilizando Inkwell (binding LLVM de Rust)

use crate::ast::*;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::FunctionValue;
use std::path::Path;

pub struct CodeGenerator<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        Self { context, module, builder }
    }

    /// Generar código para el programa
    pub fn generate_program(&self, program: &Program) -> Result<(), String> {
        let i32_type = self.context.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_type, None);

        let basic_block = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(basic_block);

        // Mapear comandos. 
        // Idealmente aquí invocaríamos a extern functions declarados de libl400.so (ej. l400_sndpgmmsg)
        for command in &program.commands {
            self.generate_command(command, main_fn)?;
        }

        // Devolver 0 para C main
        self.builder.build_return(Some(&i32_type.const_int(0, false))).map_err(|e| e.to_string())?;

        Ok(())
    }

    fn generate_command(&self, cmd: &Command, _main_fn: FunctionValue) -> Result<(), String> {
        // En un futuro, emitiremos subrutinas LLVM completas interpretando el AST de OS/400
        // Por ahora, generamos un mapeo muy general que imprime/llama a la runtime.
        println!("Generando IR para: {}", cmd.name);
        Ok(())
    }

    // Guardar LLVM en un obj nativo .o
    pub fn emit_object_file(&self, path: &str) -> Result<(), String> {
        use inkwell::targets::{Target, TargetMachine, InitializationConfig, RelocMode, CodeModel};
        use inkwell::OptimizationLevel;

        Target::initialize_all(&InitializationConfig::default());
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).map_err(|e| e.to_string())?;

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                OptimizationLevel::Default,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .ok_or("No se pudo crear el TargetMachine")?;

        let o_path = Path::new(path);
        target_machine
            .write_to_file(&self.module, inkwell::targets::FileType::Object, o_path)
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
