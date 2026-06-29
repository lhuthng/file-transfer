# file-transfer

[![Release](https://img.shields.io/github/v/release/your-username/file-transfer?label=download)](https://github.com/your-username/file-transfer/releases/latest)

Share a directory over HTTP. One host runs the server - **clients use curl** (no install needed).

## Quick start

### Download

Grab the latest binary for your OS from the [Releases page](https://github.com/your-username/file-transfer/releases/latest):

| File | Platform |
|---|---|
| `file-transfer-x86_64-linux` | Linux |
| `file-transfer-x86_64-macos` | macOS |
| `file-transfer-x86_64-windows.exe` | Windows |

```bash
# Linux / macOS
chmod +x file-transfer-x86_64-*
./file-transfer-x86_64-linux --dir ~/shared --port 9876
```

### Build from source

Requires the [Rust toolchain](https://rustup.rs/).

```bash
cargo build --release
./target/release/file-transfer --dir ~/shared --port 9876
```

## Usage

Once the server is running, clients can browse and transfer with curl:

```bash
# List files/folders
curl http://host:9876/

# Browse a subdirectory
curl http://host:9876/docs/

# Download a file
curl http://host:9876/docs/report.pdf -O

# Upload a file
curl http://host:9876/docs/report.pdf -T report.pdf
```

### Flags

| Flag | Default | Description |
|---|---|---|
| `-d`, `--dir` | `.` | Directory to share |
| `-p`, `--port` | `9876` | Port to listen on |

## Requirements

- **Host**: any of the pre-built binaries above, or [Rust toolchain](https://rustup.rs/) to build from source
- **Client**: `curl` (every system has it)

## Build from source

```bash
cargo build --release
```

The binary is at `target/release/file-transfer`.

## Security

- Path traversal (`..`, `.`) is rejected
- Existing paths are canonicalized to prevent symlink escapes
- No encryption - use on trusted networks only

## License

MIT
