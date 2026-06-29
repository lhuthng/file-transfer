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
- No encryption — use on trusted networks

## License

MIT
