# file-transfer

[![Release](https://img.shields.io/github/v/release/lhuthng/file-transfer?label=download)](https://github.com/lhuthng/file-transfer/releases/latest)

Share a directory over HTTP. One host runs the server - **clients use curl** (no install needed).

## Quick start

### Download

Grab the latest binary for your OS from the [Releases page](https://github.com/lhuthng/file-transfer/releases/latest):

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

### macOS

macOS blocks unsigned binaries by default. Before the first run, choose one:

1. **System Settings** → **Privacy & Security** → scroll down → click **Allow Anyway** next to the file-transfer message

2. Or run this command in Terminal:
   ```bash
   xattr -d com.apple.quarantine file-transfer-x86_64-macos
   ```

3. Or right-click the file in Finder, select **Open**, and click **Open anyway**

**Firewall:** When the first client connects, macOS may show a "Do you want to allow incoming connections?" dialog. Click **Allow** - this happens only once.

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
| `--token` | — | Require `X-Token` header with this value |
| `--read-only` | `false` | Reject uploads (downloads still work) |
| `--timeout` | — | Shut down after N seconds of inactivity |

### Token

```bash
# Server
file-transfer --token mysecret --dir ~/shared

# Client
curl -H "X-Token: mysecret" http://host:9876/
curl -H "X-Token: mysecret" http://host:9876/docs/report.pdf -O
curl -H "X-Token: mysecret" http://host:9876/docs/file.txt -T file.txt
```

Requests without the matching header get a `401 Unauthorized` response.
Note: the token is sent in plaintext — use on trusted networks only.

### Read-only

```bash
file-transfer --read-only --dir ~/shared
```

`PUT` requests (uploads) are rejected with `403 Forbidden`. Downloads and browsing still work.

### Timeout

```bash
file-transfer --timeout 300 --dir ~/shared
```

The server shuts down automatically after 300 seconds (5 minutes) of no requests. Useful for one-off transfers or to avoid leaving the server running.

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
