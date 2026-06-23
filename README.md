# lunar

**LunarAST CLI — Static interface contract extraction, comparison, and GitOps sync.**

`lunar` is the central command-line engine of the LunarAST ecosystem. Built with Rust, it automates project initialization, physical AST scanning, contract difference diagnostics, and **zero-trust interactive AI patch merging** with zero command-line memorization and zero manual file copy-pasting.

---

## 🏛️ Decoupled Workspace Architecture

Following the "Thin Entrypoint, Thick Library" design paradigm, `lunar` is physically structured as a highly decoupled, modular Cargo Workspace member:

```
lunar/
├── Cargo.toml
└── src/
    ├── lib.rs            # Core library module declarations
    ├── main.rs           # Ultra-lightweight Entrypoint (<70 lines of pure routing)
    ├── commands/         # Subcommand Execution Modules (Isolated boundary)
    │   ├── mod.rs        # Commands exporter
    │   ├── scan.rs       # Static analysis triggering
    │   ├── diff.rs       # Contract diff diagnostics
    │   ├── sync.rs       # Local manual merge logic
    │   ├── pull.rs       # Non-interactive secure TCP todo pull & merge
    │   ├── serve.rs      # Local serving daemon spawner
    │   ├── interactive.rs   # State-driven interactive menu
    │   ├── setup_totp.rs    # TOTP secret generation with QR code
    │   ├── visibility.rs    # Project visibility manager (lock/unlock/toggle)
    │   ├── sync_visibility.rs # GitHub API visibility sync
    │   └── sync_repos.rs      # Git metadata extraction for repos.json
    ├── map.rs            # Topography mapping & auto path discovery
    ├── cleanup.rs        # Local garbage & archive purging
    ├── doctor.rs         # Environment and consistency check-up
    ├── keygen.rs         # Cryptographic Ed25519 key-pair generator
    ├── guide.rs          # State-aware interactive menu utility
    └── uploader.rs       # S3/R2 upload client
```

---

## ⚡ Quick Start (Out-of-the-Box UX)

### 1. Installation

#### Option A: Download a pre-built binary (fastest)
Pre-built binaries are available for **Linux amd64** on the [GitHub Releases](https://github.com/LunarAST/lunar/releases) page.

1. Download `lunar` and the accompanying `checksums.txt` from the latest release.
2. (Recommended) Verify integrity:
   ```bash
   sha256sum -c checksums.txt
   ```
3. Make the binary executable and move it to a directory in your `PATH`:
   ```bash
   chmod +x lunar
   sudo mv lunar /usr/local/bin/
   ```

> **Note**: Currently only Linux amd64 is provided. For macOS, Windows, or ARM systems, please compile from source (see below).

#### Option B: Compile from source
In the `/opt/LunarAST/lunar` directory, compile and install globally:
```bash
cargo install --path .
```

### 2. Zero-Friction Interactive Menu
Simply type `lunar` from **anywhere** on your VPS/PC terminal without any flags:
```bash
lunar
```

#### The Parameter Dashboard on Boot:
* **Active Port**: Loaded from configuration or `LUNAR_SERVE_PORT` (defaults to `8787` with interactive confirmation).
* **Active Domain**: Resolves to your primary server address (e.g., `https://your-domain.com` or local fallback). Can be configured via the interactive menu (`D`) or the `LUNAR_SERVE_DOMAIN` environment variable.
* **Workspace Root**: Your current terminal physical path.
* **TOTP Status**: Shows whether the two-factor authentication secret is configured.

#### 🔔 Active Context-Aware Auto-Probe:
Upon boot, the CLI automatically scans the global `lunar-map.json` and probes all active project todo lists. If a pending contract patch is discovered, it bypasses the menu and displays an eye-catching alert:
```
🌙 LunarAST — Ecosystem Contract Governance

  🔔 Detected pending AI patch for project 'cellrix' (already reviewed via web)
     → Auto-merge and refresh map? [Y/n]: 
```
Press **Enter**! The CLI automatically pulls the patch, verifies its **Ed25519 cryptographic signature**, merges it into your local `interfaces.yml`, backs up your old file, marks the task as completed, and automatically compiles the map. 

Your public `lunar-scope` canvas is instantly updated in 0.5 seconds!

---

## 🆕 v3.0 Security & Automation Extensions

### 1. Identity Setup (`setup-totp`)
```bash
lunar setup-totp
```
Generates a Base32 TOTP secret and displays an ASCII QR code for binding to your authenticator app (Google Authenticator, Authy, etc.). The secret is stored in `.lunar/totp.secret` with `600` permissions. If a secret already exists, the command requires the current TOTP code before allowing rotation.

### 2. Visibility Manager (`visibility`)
```bash
lunar visibility
```
Interactive submenu to:
- **Lock all** – set all projects to private
- **Unlock all** – set all projects to public
- **Toggle one** – flip visibility for a specific project
- **Sync from GitHub** – pull visibility status via GitHub API (requires `GITHUB_TOKEN`)

### 3. Repository Metadata Sync (`sync-repos`)
```bash
lunar sync-repos
```
Reads `lunar-map.json` to discover all project paths, then extracts **GitHub owner, repository name, and current branch** from each project's local `.git` config. Results are written to `repos.json`, enabling automatic branch correction when generating LCT tokens.

### 4. Server Process Management (Interactive Menu)
Inside the interactive menu (after scan data is available):
- Press `5` to **start** `lunar-serve`
- Press `8` to **stop** the running server (sends `SIGTERM` via PID file)
- Press `9` to **restart** the server

No more manual `pkill -f lunar-serve`.

### 5. New Environment Variables
- `GITHUB_TOKEN` – Personal access token for syncing visibility from GitHub (optional)
- `LUNAR_SERVE_PORT` – Override default serve port (8787)
- `LUNAR_SERVE_DOMAIN` – Public domain used in token URLs (e.g., `https://your-domain.com`)

---

## 🔒 Automated Cleanups

To prevent local archives and audit logs from infinite growth, execute the explicit, on-demand cleanup command:
```bash
lunar cleanup --archive --days 30
```
This cleanly purges historical daily-rolled JSONL audit files and task archives older than the specified retention period (defaults to 30 days), respecting your disk space and data control.

---

## 📜 License

Apache-2.0
