use clap::Parser;
use l400::catalog_object;
use std::path::Path;
use std::process::Command;

fn resolve_l400_lib_path() -> String {
    if let Ok(path) = std::env::var("L400_LIB_PATH") {
        return path;
    }

    for candidate in [
        "/lib/l400",
        "/opt/l400/lib",
        "target/release",
        "target/debug",
    ] {
        let candidate_path = Path::new(candidate);
        if candidate_path.join("libl400.a").exists() || candidate_path.join("libl400.so").exists()
        {
            return candidate.to_string();
        }
    }

    String::from("target/debug")
}

/// Compilador Híbrido C/400 nativo de Linux/400
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Archivo fuente en C (.c)
    #[arg(short, long)]
    input: String,

    /// Archivo destino en ZFS
    #[arg(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();

    println!("=== Compilación C/400 ===");
    println!("Fuente : {}", args.input);
    println!("Destino: {}", args.output);

    let output_path = Path::new(&args.output);
    if !args.output.starts_with("/l400/") {
        println!(
            "  [WARN] La ruta destino '{}' no está bajo /l400/. ZFS y LSM podrían ignorarla.",
            args.output
        );
    }

    // Paso 1: Shell-Out a Clang (o cc como fallback) para generar el binario
    println!(">> (1) Resolviendo AST e inyectando runtime L400 via compilador C...");

    // Detectar compilador disponible: clang preferido, cc como fallback
    let c_compiler = if std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok()
    {
        "clang"
    } else {
        "cc"
    };
    println!("   Usando compilador: {}", c_compiler);

    let lib_path = resolve_l400_lib_path();

    let compile_status = Command::new(c_compiler)
        .arg(&args.input)
        .arg("-o")
        .arg(&args.output)
        .arg(format!("-L{}", lib_path))
        .arg(format!("-Wl,-rpath,{}", lib_path))
        .arg("-ll400")
        .status();

    match compile_status {
        Ok(status) if status.success() => {
            println!("   [OK] Artefacto nativo ELF generado exitosamente.");
        }
        _ => {
            eprintln!("   [ERROR] Fase de compilación abortada por fallas del linker o llvm!");
            std::process::exit(1);
        }
    }

    // Paso 2: Catalogación estricta ZFS
    println!(">> (2) Integración Single-Level Storage (zfs xattr)...");
    match catalog_object(output_path, "*PGM", Some("C"), Some("C/400 compiled program")) {
        Ok(_) => {
            println!("   [OK] Tipificación ZFS completada (*PGM asignado).");
        }
        Err(e) => {
            eprintln!("   [ERROR] Falló la inserción de metadatos ZFS: {}", e);
            std::process::exit(1);
        }
    }

    println!("=== Creación Completada ===");
}
