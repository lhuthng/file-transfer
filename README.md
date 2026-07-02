# file-transfer

[![Release](https://img.shields.io/github/v/release/lhuthng/file-transfer?label=download)](https://github.com/lhuthng/file-transfer/releases/latest)

Serve a directory over HTTP. Client uses **curl** — nothing to install.

```bash
# One host runs:
file-transfer --dir ~/shared --port 9876

# Any client:
curl http://host:9876/           # list files
curl http://host:9876/docs/      # list a folder
curl http://host:9876/docs -O    # download a folder as zip
curl http://host:9876/file.pdf -O  # download a file
curl http://host:9876/photo.jpg -T photo.jpg  # upload
```

> **Trailing slash matters:** `/docs/` lists contents, `/docs` downloads as zip. No trailing slash on a folder = zip download.

## Scope

`file-transfer` is meant for quick, lightweight file sharing on a trusted network.

Good fit:

- Send files between your own machines
- Share a folder with a few teammates on the same LAN/VPN
- Temporary transfers where `curl` is already available
- Small number of overlapping users

Not the target:

- Public internet file hosting
- Large multi-team deployments
- Long-running storage or sync service
- Fine-grained auth, TLS termination, or audit-heavy environments

## Targets

This project optimizes for:

- Zero-install client UX with `curl`
- Simple behavior that is easy to explain and debug
- Reliable large file transfer
- Low memory usage for directory downloads
- Small-user concurrency without pulling in a full async stack

This project does **not** currently optimize for:

- High connection counts
- Browser-first UI
- Enterprise security features
- Resumable downloads or partial-content support

## Install

Download the binary for your OS from the [Releases page](https://github.com/lhuthng/file-transfer/releases/latest):

| File | Platform |
|---|---|
| `file-transfer-x86_64-linux` | Linux |
| `file-transfer-x86_64-macos` | macOS |
| `file-transfer-x86_64-windows.exe` | Windows |

Or build from source: `cargo build --release` (requires [Rust](https://rustup.rs/)).

### macOS

macOS blocks unsigned binaries. Fix with:

```bash
xattr -d com.apple.quarantine file-transfer-x86_64-macos
```

Or go to **System Settings → Privacy & Security → Allow Anyway**.

When the first client connects, click **Allow** in the firewall dialog — once.

## Flags

| Flag | Default | Description |
|---|---|---|
| `-d`, `--dir` | `.` | Directory to share |
| `-p`, `--port` | `9876` | Port to listen on |
| `--token` | — | Require `X-Token` header with this value |
| `--read-only` | — | Reject uploads |
| `--timeout` | — | Shut down after N seconds idle |

### Token

```bash
file-transfer --token mysecret --dir ~/shared
curl -H "X-Token: mysecret" http://host:9876/
```

Missing or wrong token returns `401`. Sent in plaintext — use on trusted networks only.

### Read-only

```bash
file-transfer --read-only --dir ~/shared
# uploads → 403 Forbidden, everything else works
```

### Timeout

```bash
file-transfer --timeout 300 --dir ~/shared
# shuts down after 5 minutes of no requests
```

## Performance Notes

- File downloads stream directly from disk
- Uploads stream directly to disk
- Directory downloads are zipped and streamed using temporary disk storage, so large folders do not need to fit in memory first
- The server can handle a small number of overlapping users concurrently

## Path behavior

**Trailing slash is the key:**

| `GET /docs/` | `GET /docs` | `GET /file.pdf` |
|---|---|---|
| Lists files inside `docs/` | Downloads `docs.zip` (recursive) | Downloads the file |

- `/` alone always lists the root directory
- `/docs/` with trailing slash → browse folder contents
- `/docs` without trailing slash → download the whole folder as a zip
- `/file.pdf` → download the file

## Security

- Path traversal (`..`, `.`) rejected
- Symlinks canonicalized to prevent escape
- Optional shared-token auth via `X-Token`
- No encryption — use on trusted networks, or put it behind something that provides TLS

## Development

```bash
cargo check
cargo test
cargo build --release
```

## License

MIT
