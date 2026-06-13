# 🌙 LunarAST AI-GitOps Operations & Alignment Protocol

This manual governs the zero-friction, cryptographic, decentralized collaboration loop between **Human Developers** and **Autonomous AI Agents** using the LunarAST ecosystem.

---

## 🏛️ 1. Architectural Foundation & Truth Models

LunarAST implements a **Three-Tier Source-of-Truth Model** to govern interface contract state transitions without runtime monitoring or performance overhead:

1. **Physical Facts (`.interfaces-autogen.json`)**: Raw route metadata extracted automatically from code syntax trees. This file is excluded via `.gitignore`.
2. **Intent Overlay (`interfaces.yml`)**: Human-maintained, Git-tracked developer intent overrides. It is protected by the **Partial Field Override** algorithm, which safely merges AI-generated fields into intent configurations without altering manual human annotations or comments.
3. **Escape Hatch**: Inline directive comments (`// lunar:consume`) placed above complex dynamic runtime calls that cannot be captured by static AST parsers.

---

## 🧭 2. Human Local Workspace Loop (Zero-Friction CLI UX)

LunarAST CLI provides simplified, Codex-inspired workflow commands. Every time you modify your code locally on the VPS/PC, just run:

```bash
lunar
```

Typing `lunar` without arguments initiates the **Interactive Menu**. It displays critical boot-screen parameters in real-time:
* **Active Port**: Dynamically loaded from configuration or `LUNAR_SERVE_PORT` (defaults to `8787` with interactive confirmation).
* **Active Domain**: Resolves to your primary server address (e.g., `https://lunar.aifify.com` or local fallback).
* **Workspace Root**: Your current terminal physical path.

### Central Operations in the Menu:
*   `[1] Scan project`: Extracts raw physical facts.
*   `[5] Launch serving daemon`: Spawns the decoupled `lunar-serve` backend to host the web canvas and API routes.
*   `[6] Generate topology`: Compiles `lunar-map.json`. It automatically auto-detects absolute workspace directory paths (e.g., `/opt/cellrix`) and embeds them into the map, eliminating manual path mapping.

---

## 🤖 3. Bootstrapping AI Agents (Case-Insensitive Normalization)

When starting a conversation with any external AI Agent (e.g., GPT-4o, Claude 3.5), simply provide them your project’s `/tree` URL:
👉 `https://lunar.aifify.com/Jasonmilk/cellrix/tree/rs2`

The server automatically maps the request. To prevent URL casing friction, the gateway normalizes all coordinate lookups to **lowercase** (`jasonmilk/cellrix/rs2`).

---

## 🛠️ 4. AI On-Demand Exploration & Self-Growing Contracts

Once the AI Agent accesses the `/tree` endpoint, it instantly gains **holistic visual capabilities** and begins executing the following GitOps loop:

### Step 1: Read the System Instruction & File Tree
The AI reads the response payload of `/tree/rs2`. The server dynamically prepends the **AI Agent System Instruction** (decoupled inside your repository as `.lunar/ai-instruction.md` to avoid hardcoding) at the very top of the page.
*   **The AI learns**: It reads the `# Repository` comment headers embedded inside the file tree code block, instantly discovering how to read the project manual, use the `/raw` endpoints, and access the active todo handover lists.
*   **The AI explores**: It browses the noise-filtered file tree at the bottom (with built-in ignore lists that completely hide `.pyc`, `.venv`, and `target` directories to save Tokens).

### Step 2: Precise File Reading
The AI does NOT guess paths. It uses the file tree to make precise, on-demand HTTP GET requests to read specific files:
👉 `GET https://lunar.aifify.com/Jasonmilk/cellrix/raw/rs2/crates/cellrix-core/src/lib.rs`

### Step 3: Contract Modification & Task Handover
After implementing code features, the AI realizes a contract change is needed. It automatically generates a standard YAML contract patch, signs it cryptographically using its **Ed25519 Private Key**, and uploads it as a handover task via:
👉 `POST https://lunar.aifify.com/api/v1/projects/cellrix/todo`

---

## 🔒 5. Human-in-the-Loop Audit & One-Click Merge

To prevent unauthorized remote writes or AI hallucinations, the AI cannot directly edit your `interfaces.yml`. You maintain 100% control of the final merge:

### Step 1: AI-on-AI Peer Reviewing
Before merging, copy the URL of the comparative diff and feed it directly to another AI auditor (e.g., Claude):
👉 `https://lunar.aifify.com/api/v1/projects/cellrix/todo/diff`
The auditor AI inspects the side-by-side active contract vs. proposed patch and audits it for potential `MethodMismatch` or `Orphaned` regressions.

### Step 2: One-Click Auto-Probe & Merge (The Codex UX)
Go to your VPS terminal. You do not need to `cd` to the project directory or type complex parameters. Just run:
```bash
lunar
```
*   **Auto-Probe**: The CLI automatically scans your local map, discovers the pending todo patch, and prompts you directly on boot:
    `🔔 Detected pending AI patch for project 'cellrix' (already reviewed via web)`
    `→ Auto-merge and refresh map? [Y/n]: `
*   **Action**: Press **Enter**! 
    The CLI pulls the patch, verifies its **Ed25519 signature**, merges it into your local `interfaces.yml`, backs up your old file, marks the task as `completed` on the server, and automatically compiles the map!
*   **Result**: Refresh `https://lunar.aifify.com/`. The project node’s Exposed ports are instantly lit on the 3D canvas!

---

## 🛡️ 6. Zero-Trust Atomic Rollbacks

If any error occurs post-merge, you can restore your active branch to a 100% clean state in 1ms:
*   **Option 1 (Ecosystem Backup Copy)**:
    `cp $(ls -t .lunar/.backup/interfaces.yml.bak.* | head -n 1) .lunar/interfaces.yml`
*   **Option 2 (Git-Native Restore)**:
    `git checkout .lunar/interfaces.yml`
