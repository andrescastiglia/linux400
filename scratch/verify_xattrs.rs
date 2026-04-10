use xattr;
use std::path::Path;

fn main() {
    let files = ["tests/hola_mundo.pgm", "tests/prueba.pgm"];
    for file in &files {
        let path = Path::new(file);
        if !path.exists() {
            println!("{}: NOT FOUND", file);
            continue;
        }
        match xattr::get(path, "user.l400.objtype") {
            Ok(Some(val)) => {
                println!("{}: user.l400.objtype = {:?}", file, String::from_utf8_lossy(&val));
            }
            Ok(None) => {
                println!("{}: user.l400.objtype NOT SET", file);
            }
            Err(e) => {
                println!("{}: ERROR reading xattr: {}", file, e);
            }
        }
    }
}
