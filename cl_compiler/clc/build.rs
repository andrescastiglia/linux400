use std::process::Command;
use std::env;

fn main() {
    // Solo actuamos si la feature llvm-backend está habilitada
    if env::var("CARGO_FEATURE_LLVM_BACKEND").is_ok() {
        println!("cargo:rerun-if-env-changed=LLVM_CONFIG_PATH");
        
        // Obtener LDFLAGS de llvm-config
        let ldflags_output = Command::new("llvm-config")
            .arg("--ldflags")
            .output()
            .expect("Failed to execute llvm-config --ldflags");
        
        let ldflags = String::from_utf8_lossy(&ldflags_output.stdout);
        for flag in ldflags.split_whitespace() {
            if flag.starts_with("-L") {
                println!("cargo:rustc-link-search=native={}", &flag[2..]);
            }
        }

        // Obtener librerías compartidas de llvm-config
        let libs_output = Command::new("llvm-config")
            .arg("--libs")
            .arg("--link-shared")
            .arg("all")
            .output()
            .expect("Failed to execute llvm-config --libs --link-shared all");
        
        let libs = String::from_utf8_lossy(&libs_output.stdout);
        for lib in libs.split_whitespace() {
            if lib.starts_with("-l") {
                // Remove prefix -l
                println!("cargo:rustc-link-lib=dylib={}", &lib[2..]);
            }
        }
        
        // También necesitamos las librerías del sistema que LLVM requiere
        let system_libs_output = Command::new("llvm-config")
            .arg("--system-libs")
            .output()
            .expect("Failed to execute llvm-config --system-libs");
            
        let system_libs = String::from_utf8_lossy(&system_libs_output.stdout);
        for lib in system_libs.split_whitespace() {
            if lib.starts_with("-l") {
                println!("cargo:rustc-link-lib=dylib={}", &lib[2..]);
            }
        }
    }
}
