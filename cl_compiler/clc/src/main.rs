pub mod ast;
pub mod codegen;
pub mod compiler;
pub mod parser;

use clap::Parser;
use l400::zfs::set_objtype;
use std::path::Path;
use std::process::Command;

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
    println!(
        "Llamando al linker para resolver dependencias a {}",
        args.output
    );
    let lib_path =
        std::env::var("L400_LIB_PATH").unwrap_or_else(|_| "../libl400/target/debug".to_string());

    let link_status = Command::new("cc")
        .arg(&obj_output)
        .arg("-o")
        .arg(&args.output)
        .arg(format!("-L{}", lib_path))
        .arg("-ll400")
        // .arg("-ldb") // Integración futura con BDB real
        .status();

    match link_status {
        Ok(status) if status.success() => {
            println!("Proceso Completo de C/C++ Linker!");

            // 3. Catalogación estricta ZFS
            println!(">> (3) Integrando al Single-Level Storage (zfs xattr)...");

            let output_path = Path::new(&args.output);
            if !args.output.starts_with("/l400/") {
                println!("  [WARN] La ruta destino '{}' no está bajo /l400/. ZFS/LSM ignorará este binario.", args.output);
            }

            match set_objtype(output_path, "*PGM") {
                Ok(_) => {
                    println!("✔ Objeto nativo L400 creado en '{}'", args.output);
                }
                Err(e) => {
                    eprintln!("✘ Falla al estampar metadatos ZFS: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("✘ Falla de linking final!");
        }
    }
}
