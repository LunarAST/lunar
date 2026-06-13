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
```

---

### 📂 2. `lunar-serve` 只读分发服务层官方文档

*   **物理文件路径**：`/opt/LunarAST/lunar-serve/README.md`
*   **英文版完整内容**：

```markdown
# lunar-serve

**A highly decoupled, warning-free, read-only HTTP distribution layer for LunarAST ecosystem data.**

`lunar-serve` is a lightweight, zero-dependency HTTP server built with Axum. It consumes `lunar-map.json` and serves both human-readable Markdown contracts, structured JSON, and **on-demand raw source code mirroring** directly to AI agents with zero manual configuration.

---

## 🏛️ Decoupled Architecture

`lunar-serve` is strictly decoupled from CLI-specific operations (I/O, S3 SDKs, cryptography, terminal prompts), relying purely on the lightweight `lunar-interface` core model:

```
lunar-serve/
├── Cargo.toml
└── src/
    ├── lib.rs            # Project registry deserialization & case-insensitive matching
    ├── main.rs           # Minimalist entrypoint and controller handlers (<150 lines)
    ├── render.rs         # Markdown, Mermaid, and collapsible directory tree renderer
    └── utils.rs          # Safe file IO, JSONL rolling writer, and daily log purger
```

---

## ⚡ Quick Deployment

Ensure you have generated the global topography map using the `lunar` CLI first.

### Option 1: One-Click Launch via lunar CLI (Recommended)
In any terminal path, simply run:
```bash
lunar
```
And select `[5] Launch serving daemon` from the menu.

### Option 2: Run directly from binary
```bash
lunar-serve /opt/lunar-map.json
```
*   **Default Port**: Listens on `http://0.0.0.0:8787`.
*   **Port Override**: Export `LUNAR_SERVE_PORT=8080` to override.
*   **Host Domain Mapping**: Export `LUNAR_SERVE_DOMAIN="https://lunar.aifify.com"` to declare your primary public domain. The server will dynamically fall back to HTTP Host header sniffing if this is omitted.

---

## 🤖 AI Native Endpoints & Multimodal Negotiation

`lunar-serve` is designed as a secure, stateless, read-only code interface for AI Agentic consumption.

### 1. Canvas Index (GET `/`)
Serves the single-file compiled React canvas of `lunar-scope` natively. Opening `http://127.0.0.1:8787/` in your browser instantly loads the 3D topology chart, with zero CORS friction and zero Nginx static hosting configurations required.

### 2. File Tree & Contract Summary (Accept-based Multimodal Response)
*   **Endpoint**: `GET /:owner/:repo/tree/:branch`
*   **Behavior**: 
    *   **Default (Markdown)**: Returns a beautifully formatted RouteAST contract summary, a collapsible active todo list, and a **recursive file directory tree of your local VPS workspace** (excluding build noise like `target/`, `node_modules/`, and `.pyc` files). Prepend `#` comments inside the tree code block to guide AI navigation.
    *   **Negotiated (JSON)**: If requested with `Accept: application/json`, it bypasses markdown rendering and returns a high-cohesion JSON array containing all clean relative file paths for easy programmatic parsing.

### 3. On-Demand Source Code Reading (Zero-Copy Raw Streaming)
*   **Endpoint**: `GET /:owner/:repo/raw/:branch/*filepath` (or `/blob/` alias)
*   **Behavior**: Returns the raw text of the requested file securely. It automatically checks for directory traversal attacks via canonical path comparison and validates access permissions.

### 4. AI Handover Todo Scratchpad
*   **GET `/api/v1/projects/:name/todo`**: Retrieves the current AI task list and pending contract patches.
*   **POST `/api/v1/projects/:name/todo`**: Updates tasks and registers cryptographic handovers.
*   **GET `/api/v1/projects/:name/todo/diff`**: Returns a side-by-side comparative Markdown diff of the pending patch against your current contract for peer-review AI models.

---

## 📜 License

Apache-2.0
