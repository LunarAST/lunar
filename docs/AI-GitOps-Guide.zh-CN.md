# 🌙 LunarAST AI-GitOps 极简操作指南

本手册指导人类开发者与 AI 代理，如何通过 **`lunar-serve` 只读分发层**、**`lunar-scope` 拓扑画布** 以及 **本地 `lunar` 命令行** 开展零摩擦、无感知、100% 安全且支持回滚的 AI 协同开发。

---

## 🧭 1. 人类开发者本地操作流 (The 1-Minute Developer Flow)

每当您在 VPS 或本地电脑上更新了代码：

```bash
# 步骤 1：进入对应项目（如 helix-mind）根目录进行静态提取
cd /opt/helix-mind && lunar scan

# 步骤 2：退到全局更新世代地图，自动把最新的物理事实与工作区路径对齐
lunar map -o /opt/lunar-map.json
```

---

## 🤖 2. 引导 AI 睁眼与自组织协同 (How to Guide Any AI Agent)

在与任何联网 AI（如 GPT-4o、Claude 3.5）对话时，直接投喂以下 **AI 开发者心智引导提示词**：

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

## 🛠️ 3. AI 辅助完善 `interfaces.yml` 补丁流水线 (AI-Assisted Contract Synthesis)

### 第一阶段：AI 自动写入 Todo
AI 在通过 `/raw/` 读完代码后，发现契约缺失。AI 自动在后台向 VPS 写入它的 YAML 契约补丁提案：
*   **请求**：`POST https://lunar.aifify.com/api/v1/projects/helix-mind/todo`

### 第二阶段：交叉模型 AI 同行评审（AI-on-AI Auditing）
在人类正式确认合并前，人类可以将以下端点直接丢给另一个 AI 审计师（如 Claude 3.5）：
*   **地址**：`https://lunar.aifify.com/api/v1/projects/helix-mind/todo/diff`
*   **指令**：*“Please fetch this URL to examine the proposed YAML contract patch and audit it for potential MethodMismatch or Orphaned risks.”*

### 第三阶段：人类 1 秒一键对齐（Pull-to-Align）
在 VPS 终端，人类运行极其高雅的单行命令，自动安全拉取并拉起终端评审合并器 [4.3]：
```bash
lunar sync --from-todo
```
*   CLI 自动显示 Diff。
*   输入 `y` 确认合并。

---

## 🛡️ 4. 原子化安全回滚保障 (The Rollback Guarantee)

如果合并后发现 AI 的补丁有缺陷，想要撤销：

### 方案 1：系统自动物理备份还原（零残留） [4.3]
```bash
cp $(ls -t .lunar/.backup/interfaces.yml.bak.* | head -n 1) .lunar/interfaces.yml
```

### 方案 2：Git 原生恢复 [2.2]
```bash
git checkout .lunar/interfaces.yml
```
```

