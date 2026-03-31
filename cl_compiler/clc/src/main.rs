pub mod ast;
pub mod parser;
pub mod codegen;
pub mod compiler;

use std::process::Command;
use clap::Parser;

/// Compilador de Control Language (CL) nativo de Linux/400
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Archivo de fuente .clp
    #[arg(short, long)]
    input: String,

    /// Archivo final generado compilado
    #[arg(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();
    println!("=== Compilando {} ===", args.input);

    let obj_output = format!("{}.o", args.output);
    
    // 1. AST -> IR -> Objeto
    match compiler::Compiler::compile(&args.input, &obj_output) {
        Ok(_) => println!("✔ Código objeto generado en {}", obj_output),
        Err(e) => {
            eprintln!("✘ Falla al compilar: {}", e);
            std::process::exit(1);
        }
    }

    // 2. Linking Objecto -> libL400.so -> Runtime Ejecutable
    // Enlazar temporal obj con la libreria core de linux/400 (depende del compilador C cc)
    println!("Llamando al linker para resolver dependencias a {}", args.output);

    let link_status = Command::new("cc")
        .arg(&obj_output)
        .arg("-o")
        .arg(&args.output)
        .arg("-L../libl400/target/debug") // Asumiendo path dev 
        .arg("-ll400")
        // .arg("-ldb") // Integración futura con BDB real
        .status();

    match link_status {
        Ok(status) if status.success() => {
            println!("Proceso Completo! Artefacto listo en '{}'", args.output);
        }
        _ => {
            eprintln!("Falla de linking final!");
        }
    }
}
