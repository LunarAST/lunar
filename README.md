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
    │   └── serve.rs      # Local serving daemon spawner
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
* **Active Domain**: Resolves to your primary server address (e.g., `https://lunar.aifify.com` or local fallback).
* **Workspace Root**: Your current terminal physical path.

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

## 🔒 Automated Cleanups

To prevent local archives and audit logs from infinite growth, execute the explicit, on-demand cleanup command:
```bash
lunar cleanup --archive --days 30
```
This cleanly purges historical daily-rolled JSONL audit files and task archives older than the specified retention period (defaults to 30 days), respecting your disk space and data control.

---

## 📜 License

Apache-2.0
