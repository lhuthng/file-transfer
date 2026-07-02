use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use tiny_http::{Header, Response, ResponseBox};
use zip::ZipWriter;

pub(super) fn stream_directory_zip(
    display_name: &str,
    dir: &Path,
) -> Result<ResponseBox, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let display_name = display_name.to_string();
    let root = dir.to_path_buf();
    let temp_path = temp_zip_path();
    let writer = OpenOptions::new()
        .create_new(true)
        .write(true)
        .read(true)
        .open(&temp_path)?;
    let reader = OpenOptions::new().read(true).open(&temp_path)?;
    let done = Arc::new(AtomicBool::new(false));
    let done_for_thread = Arc::clone(&done);
    let error_name = display_name.clone();

    thread::Builder::new()
        .name(format!("zip-stream-{}", sanitize_thread_name(&display_name)))
        .spawn(move || {
            if let Err(err) = write_zip_stream(&root, writer) {
                eprintln!("Zip stream error for {}: {}", error_name, err);
            }
            done_for_thread.store(true, Ordering::Release);
        })?;

    let content_disposition = format!("attachment; filename=\"{}.zip\"", display_name);
    let response_reader = GrowingFileReader::new(reader, temp_path, done);
    let response = Response::empty(200)
        .with_data(response_reader, None)
        .with_header(content_type("application/zip"))
        .with_header(
            Header::from_bytes(&b"Content-Disposition"[..], content_disposition.as_bytes())
                .unwrap(),
        )
        .boxed();

    Ok(response)
}

fn write_zip_stream(
    dir: &Path,
    writer: File,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut zip = ZipWriter::new(writer);
    add_dir_to_zip(&mut zip, dir, "")?;
    let mut writer = zip.finish()?;
    writer.flush()?;
    Ok(())
}

fn add_dir_to_zip<W: io::Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    dir: &Path,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    for entry in fs::read_dir(dir)?.flatten() {
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let path = entry.path();
        let relative = if prefix.is_empty() {
            name.to_string_lossy().to_string()
        } else {
            format!("{}/{}", prefix, name.to_string_lossy())
        };

        let options = zip::write::SimpleFileOptions::default();
        if file_type.is_dir() {
            zip.add_directory(&relative, options)?;
            add_dir_to_zip(zip, &path, &relative)?;
        } else if file_type.is_file() {
            zip.start_file(&relative, options)?;
            let mut file = fs::File::open(&path)?;
            io::copy(&mut file, zip)?;
        }
    }
    Ok(())
}

fn content_type(mime: &str) -> Header {
    Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap()
}

fn sanitize_thread_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .take(32)
        .collect()
}

fn temp_zip_path() -> PathBuf {
    let unique = format!(
        "file-transfer-zip-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    std::env::temp_dir().join(unique)
}

struct GrowingFileReader {
    file: File,
    path: PathBuf,
    done: Arc<AtomicBool>,
}

impl GrowingFileReader {
    fn new(file: File, path: PathBuf, done: Arc<AtomicBool>) -> Self {
        Self { file, path, done }
    }
}

impl Read for GrowingFileReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let bytes_read = self.file.read(buf)?;
            if bytes_read > 0 {
                return Ok(bytes_read);
            }

            if self.done.load(Ordering::Acquire) {
                let pos = self.file.stream_position()?;
                let len = self.file.metadata()?.len();
                if pos >= len {
                    return Ok(0);
                }
            }

            thread::sleep(Duration::from_millis(10));
            self.file.seek(SeekFrom::Current(0))?;
        }
    }
}

impl Drop for GrowingFileReader {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
