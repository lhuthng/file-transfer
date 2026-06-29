# file-transfer

Share a directory over HTTP. One host runs the server - **clients use curl** (no install needed).

## Quick start

```bash
# Build
cargo build --release

# Host a directory
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

- [Rust toolchain](https://rustup.rs/) (rustc + cargo) - only needed on the **host** machine
- `curl` - any machine that has it (every system)

## Build

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
