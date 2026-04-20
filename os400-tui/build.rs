fn main() {
    println!("cargo:rustc-link-arg=-Wl,-l:libpam.so.0");
}
