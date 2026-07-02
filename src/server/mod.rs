mod paths;
mod worker_pool;
mod zip_stream;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use self::paths::safe_path;
use self::worker_pool::WorkerPool;
use self::zip_stream::stream_directory_zip;

pub fn serve(
    shared_dir: &Path,
    addr: &str,
    token: Option<String>,
    read_only: bool,
    idle_timeout: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let server = Server::http(addr)?;
    let shared_dir = Arc::new(shared_dir.to_path_buf());
    let token = Arc::new(token);
    let workers = worker_pool::worker_count();
    let dispatcher = WorkerPool::new(
        workers,
        Arc::clone(&shared_dir),
        Arc::clone(&token),
        read_only,
    );

    println!("  Note: macOS firewall popup appears on first client connection - click Allow (one-time).");
    println!("  Workers: {}", workers);

    match idle_timeout {
        Some(secs) => serve_with_timeout(&server, &dispatcher, secs),
        None => serve_forever(&server, &dispatcher),
    }
}

fn serve_forever(
    server: &Server,
    dispatcher: &WorkerPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    for request in server.incoming_requests() {
        if let Err(e) = dispatcher.dispatch(request) {
            eprintln!("Error: {}", e);
        }
    }
    Ok(())
}

fn serve_with_timeout(
    server: &Server,
    dispatcher: &WorkerPool,
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

        match server.recv_timeout(timeout - elapsed) {
            Ok(Some(request)) => {
                last_activity = Instant::now();
                if let Err(e) = dispatcher.dispatch(request) {
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

pub(super) fn handle_request(
    shared_dir: &Path,
    token: &Option<String>,
    read_only: bool,
    request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let url = request.url().to_owned();
    let method = request.method().clone();

    if let Some(expected_token) = token {
        if !has_valid_token(request.headers(), expected_token) {
            return respond_plain(request, StatusCode(401), "Unauthorized");
        }
    }

    if read_only && method == Method::Put {
        return respond_plain(request, StatusCode(403), "Read-only mode");
    }

    dispatch(shared_dir, method, &url, request)
}

fn has_valid_token(headers: &[Header], expected_token: &str) -> bool {
    headers
        .iter()
        .any(|h| h.field.equiv("X-Token") && h.value.as_str() == expected_token)
}

fn dispatch(
    shared_dir: &Path,
    method: Method,
    url: &str,
    request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    match method {
        Method::Get => {
            let path = match safe_path(shared_dir, url, false) {
                Some(path) => path,
                None => return respond_plain(request, StatusCode(400), "Invalid path"),
            };

            if url.ends_with('/') {
                if path.is_dir() {
                    handle_list_dir(&path, request)
                } else {
                    respond_plain(request, StatusCode(404), "Not found")
                }
            } else if path.is_file() {
                handle_download(url, &path, request)
            } else if path.is_dir() {
                handle_zip_directory(url, &path, request)
            } else {
                respond_plain(request, StatusCode(404), "Not found")
            }
        }
        Method::Put => {
            let path = match safe_path(shared_dir, url, true) {
                Some(path) => path,
                None => return respond_plain(request, StatusCode(400), "Invalid path"),
            };
            handle_upload(url, &path, request)
        }
        _ => respond_plain(request, StatusCode(404), "Not found"),
    }
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
        let file_type = entry.file_type().ok();
        if file_type.is_some_and(|t| t.is_dir()) {
            entries.push(format!("{}/", name));
        } else if file_type.is_some_and(|t| t.is_file()) {
            entries.push(name);
        }
    }
    entries.sort();
    entries.dedup();

    let body = entries.join("\n") + "\n";
    println!("Listed {} items from {}", entries.len(), path.display());
    request.respond(Response::from_string(body))?;
    Ok(())
}

fn handle_download(
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
    let response = Response::from_file(file).with_header(content_type("application/octet-stream"));
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

fn handle_zip_directory(
    url: &str,
    path: &Path,
    request: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let display = url.trim_start_matches('/');
    println!("Streaming zip {}", display);
    let response = stream_directory_zip(display, path)?;
    request.respond(response)?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{handle_request, has_valid_token};
    use crate::server::zip_stream::stream_directory_zip;
    use tiny_http::{Header, Method, TestRequest};
    use std::fs;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("file-transfer-test-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn token_validation_accepts_only_matching_header() {
        let matching = vec![Header::from_str("X-Token: secret").unwrap()];
        let wrong = vec![Header::from_str("X-Token: nope").unwrap()];

        assert!(has_valid_token(&matching, "secret"));
        assert!(!has_valid_token(&wrong, "secret"));
        assert!(!has_valid_token(&[], "secret"));
    }

    #[test]
    fn read_only_mode_rejects_put_uploads() {
        let root = make_temp_dir();
        let request: tiny_http::Request = TestRequest::new()
            .with_method(Method::Put)
            .with_path("/upload.txt")
            .with_body("hello")
            .into();

        handle_request(&root, &None, true, request).unwrap();
        assert!(!root.join("upload.txt").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_download_uses_zip_response_headers() {
        let root = make_temp_dir();
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("note.txt"), "hello").unwrap();

        let response = stream_directory_zip("docs", &docs).unwrap();
        let headers = response.headers();

        assert_eq!(response.status_code().0, 200);
        assert!(headers.iter().any(|h| {
            h.field.equiv("Content-Type") && h.value.as_str() == "application/zip"
        }));
        assert!(headers.iter().any(|h| {
            h.field.equiv("Content-Disposition")
                && h.value.as_str() == "attachment; filename=\"docs.zip\""
        }));

        fs::remove_dir_all(root).unwrap();
    }
}
