use std::net::UdpSocket;
use std::path::PathBuf;
use clap::Parser;

mod server;

#[derive(Parser)]
#[command(name = "file-transfer", about = "Share files over HTTP. Client uses curl.")]
struct Cli {
    #[arg(short, long, default_value = ".", help = "Directory to share")]
    dir: PathBuf,

    #[arg(short, long, default_value_t = 9876, help = "Port to listen on")]
    port: u16,
}

fn local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

fn main() {
    let args = Cli::parse();

    let shared_dir = args.dir.canonicalize().unwrap_or_else(|e| {
        eprintln!("Error: cannot access directory '{}': {}", args.dir.display(), e);
        std::process::exit(1);
    });

    let addr = format!("0.0.0.0:{}", args.port);
    let port = args.port;

    println!("Serving {}", shared_dir.display());
    println!("  http://localhost:{}", port);
    if let Some(ip) = local_ip() {
        println!("  http://{}:{}", ip, port);
    }
    println!();
    println!("Browse:  curl http://localhost:{}/", port);
    println!("Download: curl http://localhost:{}/path/to/file -O", port);
    println!("Upload:  curl http://localhost:{}/path/to/file -T file.txt", port);

    if let Err(e) = server::serve(&shared_dir, &addr) {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
