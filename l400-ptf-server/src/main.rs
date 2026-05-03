use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

const PTF_CACHE_DIR: &str = "/var/cache/l400/ptf";

/// Simple PTF server that serves PTF packages via HTTP
/// SERVICE option only (no tapes)
fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    println!("PTF server listening on port 8080");
    println!("PTF cache directory: {}", PTF_CACHE_DIR);
    println!("SERVICE option only (no tapes support)");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream) {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0; 1024];
    let _bytes_read = stream.read(&mut buffer)?;

    let request = String::from_utf8_lossy(&buffer);
    let request_line = request.lines().next().unwrap_or_default();

    if request_line.starts_with("GET /ptf/list") {
        handle_list_ptfs(&mut stream)?;
    } else if request_line.starts_with("GET /ptf/") {
        handle_get_ptf(&mut stream, request_line)?;
    } else {
        handle_not_found(&mut stream)?;
    }

    Ok(())
}

fn handle_list_ptfs(stream: &mut TcpStream) -> std::io::Result<()> {
    let cache_dir = Path::new(PTF_CACHE_DIR);
    let mut response_body = String::new();

    response_body.push_str("PTF_ID\tNAME\tVERSION\tSTATUS\n");
    response_body.push_str("----------------------------------------\n");

    if cache_dir.exists() {
        if let Ok(entries) = fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    || path
                        .extension()
                        .is_some_and(|ext| ext == "tar.gz" || ext == "tgz")
                {
                    let manifest_path = if path.is_dir() {
                        path.join("manifest.toml")
                    } else {
                        continue;
                    };

                    if manifest_path.exists() {
                        if let Ok(content) = fs::read_to_string(&manifest_path) {
                            let id = extract_toml_value(&content, "package.id")
                                .unwrap_or_else(|| "Unknown".to_string());
                            let name = extract_toml_value(&content, "package.name")
                                .unwrap_or_else(|| "Unknown".to_string());
                            let version = extract_toml_value(&content, "package.version")
                                .unwrap_or_else(|| "Unknown".to_string());

                            response_body
                                .push_str(&format!("{}\t{}\t{}\tCACHED\n", id, name, version));
                        }
                    }
                }
            }
        }
    }

    send_response(stream, 200, "text/plain", &response_body)
}

fn handle_get_ptf(stream: &mut TcpStream, request_line: &str) -> std::io::Result<()> {
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return handle_not_found(stream);
    }

    let path = parts[1];
    let ptf_id = path.trim_start_matches("/ptf/").trim();

    if ptf_id.is_empty() {
        return handle_not_found(stream);
    }

    let cache_dir = Path::new(PTF_CACHE_DIR);
    let ptf_path = cache_dir.join(ptf_id);

    if !ptf_path.exists() {
        return handle_not_found(stream);
    }

    if ptf_path.is_dir() {
        let manifest_path = ptf_path.join("manifest.toml");
        if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            return send_response(stream, 200, "text/plain", &content);
        }
    }

    handle_not_found(stream)
}

fn handle_not_found(stream: &mut TcpStream) -> std::io::Result<()> {
    send_response(stream, 404, "text/plain", "404 Not Found\n")
}

fn send_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    Ok(())
}

fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{} = ", key)) {
            if let Some(start) = line.find('"') {
                if let Some(end) = line.rfind('"') {
                    if start != end {
                        return Some(line[start + 1..end].to_string());
                    }
                }
            }
        }
    }
    None
}
