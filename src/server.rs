use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub fn serve(
    shared_dir: &Path,
    addr: &str,
    token: Option<String>,
    read_only: bool,
    idle_timeout: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let server = Server::http(addr)?;
    let shared_dir = shared_dir.to_path_buf();

    println!("  Note: macOS firewall popup appears on first client connection - click Allow (one-time).");

    match idle_timeout {
        Some(secs) => serve_with_timeout(&server, &shared_dir, &token, read_only, secs),
        None => serve_forever(&server, &shared_dir, &token, read_only),
    }
}

fn serve_forever(
    server: &Server,
    shared_dir: &Path,
    token: &Option<String>,
    read_only: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    for request in server.incoming_requests() {
        if let Err(e) = handle_request(shared_dir, token, read_only, request) {
            eprintln!("Error: {}", e);
        }
    }
    Ok(())
}

fn serve_with_timeout(
    server: &Server,
    shared_dir: &Path,
    token: &Option<String>,
    read_only: bool,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let timeout = Duration::from_secs(timeout_secs);
    let mut last_activity = Instant::now();

    loop {
        let elapsed = last_activity.elapsed();
        if elapsed >= timeout {
            println!("Idle timeout reached ({}s), shutting down", timeout_secs);
            return Ok(());
        }
        let remaining = timeout - elapsed;

        match server.recv_timeout(remaining) {
            Ok(Some(request)) => {
                last_activity = Instant::now();
                if let Err(e) = handle_request(shared_dir, token, read_only, request) {
                    eprintln!("Error: {}", e);
                }
            }
            Ok(None) => {
                println!("Idle timeout reached ({}s), shutting down", timeout_secs);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Server error: {}", e);
                return Err(e.into());
            }
        }
    }
}

fn handle_request(
    shared_dir: &Path,
    token: &Option<String>,
    read_only: bool,
    request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let url = request.url().to_string();
    let method = request.method().clone();

    if let Some(ref t) = token {
        let ok = request
            .headers()
            .iter()
            .any(|h| h.field.equiv("X-Token") && h.value.as_str() == t.as_str());
        if !ok {
            return respond_plain(request, StatusCode(401), "Unauthorized");
        }
    }

    if read_only && method == Method::Put {
        return respond_plain(request, StatusCode(403), "Read-only mode");
    }

    dispatch(shared_dir, method, &url, request)
}

fn dispatch(
    shared_dir: &Path,
    method: Method,
    url: &str,
    request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    match method {
        Method::Get => {
            let path = match safe_path(shared_dir, url) {
                Some(p) => p,
                None => return respond_plain(request, StatusCode(400), "Invalid path"),
            };

            if path.is_dir() {
                handle_list_dir(&path, request)
            } else if path.is_file() {
                handle_download(shared_dir, url, &path, request)
            } else {
                respond_plain(request, StatusCode(404), "Not found")
            }
        }
        Method::Put => {
            let path = match safe_path(shared_dir, url) {
                Some(p) => p,
                None => return respond_plain(request, StatusCode(400), "Invalid path"),
            };
            handle_upload(url, &path, request)
        }
        _ => respond_plain(request, StatusCode(404), "Not found"),
    }
}

fn safe_path(shared_dir: &Path, url: &str) -> Option<PathBuf> {
    let relative = url.trim_start_matches('/');

    if relative.split('/').any(|c| c == ".." || c == ".") {
        return None;
    }

    let path = shared_dir.join(relative);

    if path.exists() {
        let canonical = path.canonicalize().ok()?;
        let shared_canonical = shared_dir.canonicalize().ok()?;
        if canonical.starts_with(&shared_canonical) {
            return Some(canonical);
        }
        return None;
    }

    Some(path)
}

fn content_type(mime: &str) -> Header {
    Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap()
}

fn respond_plain(
    request: Request,
    status: StatusCode,
    body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let response = Response::from_string(body)
        .with_status_code(status)
        .with_header(content_type("text/plain; charset=utf-8"));
    request.respond(response)?;
    Ok(())
}

fn handle_list_dir(
    path: &Path,
    request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let ft = entry.file_type().ok();
        if ft.map_or(false, |t| t.is_dir()) {
            entries.push(format!("{}/", name));
        } else if ft.map_or(false, |t| t.is_file()) {
            entries.push(name);
        }
    }
    entries.sort();
    entries.dedup();
    let body = entries.join("\n") + "\n";
    println!("Listed {} items from {}", entries.len(), path.display());
    let response = Response::from_string(body);
    request.respond(response)?;
    Ok(())
}

fn handle_download(
    _shared_dir: &Path,
    url: &str,
    path: &Path,
    request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let display = url.trim_start_matches('/');
    let file = fs::File::open(path)?;
    println!(
        "Downloaded {} ({})",
        display,
        format_size(fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0))
    );
    let response = Response::from_file(file)
        .with_header(content_type("application/octet-stream"));
    request.respond(response)?;
    Ok(())
}

fn handle_upload(
    url: &str,
    path: &Path,
    mut request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let display = url.trim_start_matches('/');

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(path)?;
    let size = std::io::copy(&mut request.as_reader(), &mut file)?;

    println!("Uploaded {} ({})", display, format_size(size as usize));
    respond_plain(request, StatusCode(200), "OK")
}

fn format_size(size: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}
