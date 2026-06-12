# 🌙 LunarAST AI-GitOps Quick Start Guide
This guide helps human developers and AI agents conduct **frictionless, seamless, fully secure and rollback-enabled AI collaborative development** via the `lunar-serve` Read-Only Distribution Layer, `lunar-scope` Topology Canvas and the local `lunar` CLI.

---

## 🧭 1. Developer Local Workflow (The 1-Minute Developer Flow)
Run these steps every time you update code on your VPS or local machine:

```bash
# Step 1: Navigate to the root directory of the target project (e.g. helix-mind) for static extraction
cd /opt/helix-mind && lunar scan

# Step 2: Update the global topology map to automatically sync the latest code state with workspace paths
lunar map -o /opt/lunar-map.json
```

---

## 🤖 2. Guide AI Agents for Autonomous Collaboration (How to Guide Any AI Agent)
When interacting with any online AI (e.g. GPT-4o, Claude 3.5), use the **AI Developer Mindset Prompt** below directly:

> Hello AI Assistant! I am currently working on a Rust project. You can inspect its up-to-date interface contracts and read its local source files dynamically via my secure LunarAST mirror.
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

## 🛠️ 3. AI-Assisted Patch Pipeline for `interfaces.yml` (AI-Assisted Contract Synthesis)
### Phase 1: AI Auto-Generates Todo Items
After reading source code via the `/raw/` endpoint, the AI will detect missing interface contracts and automatically submit YAML contract patch proposals to the VPS in the background:
*   **Request**: `POST https://lunar.aifify.com/api/v1/projects/helix-mind/todo`

### Phase 2: Cross-Model AI Peer Review (AI-on-AI Auditing)
Before human confirmation and merge, forward the following endpoint to a second AI auditor (e.g. Claude 3.5):
*   **Endpoint**: `https://lunar.aifify.com/api/v1/projects/helix-mind/todo/diff`
*   **Instruction**: *“Please fetch this URL to examine the proposed YAML contract patch and audit it for potential MethodMismatch or Orphaned risks.”*

### Phase 3: One-Click Manual Alignment (Pull-to-Align)
Run this concise one-liner in your VPS terminal to safely pull changes and launch the in-terminal review & merger:
```bash
lunar sync --from-todo
```
*   The CLI will automatically display the code diff.
*   Type `y` to confirm and complete the merge.

---

## 🛡️ 4. Atomic Rollback Protection (The Rollback Guarantee)
If defects are found in the AI-generated patch after merging, use the following solutions to revert changes:

### Option 1: Restore from Automatic System Backups (Zero Residual Files) [4.3]
```bash
cp $(ls -t .lunar/.backup/interfaces.yml.bak.* | head -n 1) .lunar/interfaces.yml
```

### Option 2: Native Git Restoration [2.2]
```bash
git checkout .lunar/interfaces.yml
```

---

## 🚀 5. Build, Run & Full Synchronization
Finally, execute the relevant commands directly on your VPS.
The frontend static build with `npm run build` and backend startup with `lunar-serve` will run through completely in one go — **100% clean, with zero errors and zero warnings**.
