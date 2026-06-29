use std::fs;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub fn serve(
    shared_dir: &Path,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let server = Server::http(addr)?;
    let shared_dir = shared_dir.to_path_buf();

    println!("  Note: macOS firewall popup appears on first client connection - click Allow (one-time).");

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();

        if let Err(e) = dispatch(&shared_dir, method, &url, request) {
            eprintln!("Error: {} - {}", url, e);
        }
    }

    Ok(())
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
