# 🌙 LunarAST AI-GitOps Operations & Alignment Protocol

This manual governs the zero-friction, cryptographic, decentralized collaboration loop between **Human Developers** and **Autonomous AI Agents** using the LunarAST ecosystem.

---

## 🧭 1. Human Local Workspace Loop (Out-of-the-Box UX)

LunarAST CLI provides simplified, Codex-inspired workflow commands. Every time you modify your code locally on the VPS/PC, just run:

```bash
# Step 1: Scan and extract physical facts (Run inside your project, e.g., /opt/helix-mind)
lunar scan

# Step 2: Regenerate the global map (Auto-aligns workspace paths and strips build noise)
lunar map -o /opt/lunar-map.json

# Step 3: Run the lightweight HTTP serving daemon instantly
lunar serve
```

---

## 🤖 2. Bootstrapping AI Agents (Case-Insensitive Normalization)

When starting a conversation with any external AI Agent (e.g., GPT-4o, Claude 3.5), simply copy and paste this standard **AI System Instruction** to grant them instant "vision" over your VPS codebase:

> Hello AI Assistant! I am currently working on a project. You can inspect its up-to-date interface contracts and read its local source files dynamically via my secure LunarAST mirror.
> 
> Please follow these steps to "see" and "explore" my codebase:
> 
> **Step 1: Discover the File Tree**
> Fetch this URL to retrieve the contract summary and the clean, recursive physical directory tree of the project workspace (excluding noise like build directories or pyc files):
> 👉 `https://lunar.aifify.com/Jasonmilk/helix-mind/tree/rs`
> 
> **Step 2: Read Specific Files On-Demand**
> Once you inspect the directory tree from Step 1, do NOT guess. When you need to read the contents of any specific file (e.g., `crates/helix-mind-cli/src/main.rs`), simply fetch its raw content on-demand via this mirror:
> 👉 `https://lunar.aifify.com/Jasonmilk/helix-mind/raw/rs/<filepath>`
> 
> **Step 3: Read or Update the Handover TODOs**
> To check what the previous AI session left for you, or to save your current work progress for the next session, you can GET or POST (JSON payload) to our handover scratchpad:
> 👉 `https://lunar.aifify.com/api/v1/projects/helix-mind/todo`
> 
> Let's begin! Please fetch the project directory tree from Step 1, examine the folder layout, and let me know when you are ready to assist.

---

## 🛠️ 3. Decoupled AI Agent Instructions (No Hardcoding)

To allow customized prompting for different projects (e.g., `Cellrix` vs `Helix-Mind`), `lunar-serve` dynamically scans for `.lunar/ai-instruction.md` inside your project directory. 
*   **Behavior**: If `.lunar/ai-instruction.md` is found, the server automatically prepends it to the top of the `/tree` endpoint. If missing, it gracefully falls back to a clean default.
*   **Action**: Create a custom `.lunar/ai-instruction.md` inside your repository to define unique behaviors for specialized AI nodes.

---

## 🔒 4. AI-on-AI Peer Reviewing & Cryptographic Validation

### Step 1: AI Generates & POSTs Patch
The AI assistant analyzes the codebase via `/raw` and publishes its proposed contract updates to the handover todo:
*   `POST https://lunar.aifify.com/api/v1/projects/helix-mind/todo`

### Step 2: Cross-Model Auditing (Zero Copy-Paste)
Before you merge, copy the URL of the generated comparative Markdown diff and feed it directly to another AI auditor (e.g. Claude):
*   👉 `https://lunar.aifify.com/api/v1/projects/helix-mind/todo/diff`
*   **Prompt**: *“Please fetch this URL to audit the pending contract diff proposed by the previous AI model, checking for any API regressions or security risks.”*

### Step 3: One-Click Local Align & Merge
In your project directory, execute the simple, out-of-the-box pull command:
```bash
lunar pull
```
*   **Under the hood**: The CLI connects to `127.0.0.1:8787`, pulls the JSON, validates its **Ed25519 signature** to prevent malicious payload injections, and prints a beautiful Git-style Diff.
*   Confirm with `y` to apply, merge, and backup.

---

## 🛡️ 5. Zero-Trust Atomic Rollbacks

If any error occurs post-merge:

*   **Option 1 (System Backup Copy)**:
    `cp $(ls -t .lunar/.backup/interfaces.yml.bak.* | head -n 1) .lunar/interfaces.yml`
*   **Option 2 (Git-Native Restore)**:
    `git checkout .lunar/interfaces.yml`
