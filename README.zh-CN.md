# lunar

**LunarAST CLI — 静态接口契约提取、比对与 GitOps 同步工具。**

`lunar` 是 LunarAST 生态的核心命令行引擎。它基于 Rust 构建，实现了项目初始化、物理 AST 扫描、契约差异诊断以及**零信任交互式 AI 补丁合并**，无需记忆任何命令，也无需手动复制粘贴文件。

---

## 🏛️ 高度解耦的工作区架构

遵循 **“薄入口，厚库”** 的设计范式，`lunar` 在物理上被组织为高度解耦、模块化的 Cargo 工作区成员：

```
lunar/
├── Cargo.toml
└── src/
    ├── lib.rs            # 核心库模块声明
    ├── main.rs           # 极轻量入口（纯路由代码 <70 行）
    ├── commands/         # 子命令执行模块（隔离边界）
    │   ├── mod.rs        # 命令导出器
    │   ├── scan.rs       # 静态分析触发
    │   ├── diff.rs       # 契约差异诊断
    │   ├── sync.rs       # 本地手动合并逻辑
    │   ├── pull.rs       # 非交互式安全 TCP 拉取与合并
    │   └── serve.rs      # 本地守护进程启动器
    ├── map.rs            # 拓扑地图生成与自动路径发现
    ├── cleanup.rs        # 本地垃圾与归档清理
    ├── doctor.rs         # 环境与一致性检查
    ├── keygen.rs         # 密码学 Ed25519 密钥对生成器
    ├── guide.rs          # 状态感知的交互式菜单工具
    └── uploader.rs       # S3/R2 上传客户端
```

---

## ⚡ 快速开始（开箱即用的用户体验）

### 1. 安装
在 `/opt/LunarAST/lunar` 目录下，编译并全局安装：
```bash
cargo install --path .
```

### 2. 零摩擦交互式菜单
在 VPS/PC 终端的**任何路径**下，直接输入 `lunar` 即可（无需任何参数）：
```bash
lunar
```

#### 启动时的参数看板：
* **Active Port**：从配置或 `LUNAR_SERVE_PORT` 加载（默认 `8787`，支持交互确认）。
* **Active Domain**：解析为主服务地址（例如 `https://lunar.aifify.com` 或本地回退）。
* **Workspace Root**：当前终端的物理路径。

#### 🔔 主动上下文感知自动探测：
启动时，CLI 会自动扫描全局 `lunar-map.json`，并探测所有活跃项目的待办列表。一旦发现待合并的契约补丁，它会绕过菜单并显示醒目提示：
```
🌙 LunarAST — Ecosystem Contract Governance

  🔔 检测到项目 'cellrix' 存在待合并的 AI 补丁（已通过 Web 审查）
     → 自动合并并刷新地图？[Y/n]: 
```
按下 **回车**！CLI 会自动拉取补丁，验证其 **Ed25519 密码学签名**，合并到本地的 `interfaces.yml` 中，备份旧文件，标记任务为已完成，并自动编译地图。  
你的公网 `lunar-scope` 画布将在 0.5 秒内瞬间更新！

---

## 🔒 自动清理

为了防止本地归档和审计日志无限增长，可执行显式的按需清理命令：
```bash
lunar cleanup --archive --days 30
```
该命令会干净地清除超过指定保留期（默认 30 天）的历史每日滚动 JSONL 审计文件与任务归档，尊重你的磁盘空间和数据控制权。

---

## 📜 许可证

Apache-2.0
```

---

## 📂 2. `lunar-serve` 只读分发服务层官方文档（中文版）

**物理文件路径**：`/opt/LunarAST/lunar-serve/README.zh-CN.md`

```markdown
# lunar-serve

**高度解耦、零警告、只读的 LunarAST 生态数据 HTTP 分发层。**

`lunar-serve` 是一个基于 Axum 构建的轻量级、零运行时依赖的 HTTP 服务器。它消费 `lunar-map.json`，并为 AI 代理提供人类可读的 Markdown 契约、结构化 JSON 以及**按需的原始源代码镜像**，无需任何人工配置。

---

## 🏛️ 解耦架构

`lunar-serve` 与 CLI 特有的操作（I/O、S3 SDK、密码学、终端提示）严格解耦，仅依赖轻量的 `lunar-interface` 核心模型：

```
lunar-serve/
├── Cargo.toml
└── src/
    ├── lib.rs            # 项目注册表反序列化 & 大小写不敏感匹配
    ├── main.rs           # 极简入口和控制器处理程序（<150 行）
    ├── render.rs         # Markdown、Mermaid 和可折叠目录树渲染器
    └── utils.rs          # 安全的文件 IO、JSONL 滚动写入器、日志每日清理
```

---

## ⚡ 快速部署

请确保已先用 `lunar` CLI 生成全局拓扑地图。

### 方式一：通过 `lunar` CLI 一键启动（推荐）
在任意终端路径下，直接运行：
```bash
lunar
```
然后从菜单中选择 `[5] Launch serving daemon`。

### 方式二：直接从二进制运行
```bash
lunar-serve /opt/lunar-map.json
```
*   **默认端口**：监听 `http://0.0.0.0:8787`。
*   **端口覆盖**：设置环境变量 `LUNAR_SERVE_PORT=8080` 覆盖。
*   **主机域名映射**：设置 `LUNAR_SERVE_DOMAIN="https://lunar.aifify.com"` 声明你的公网主域名。如果未设置，服务器将动态回退到 HTTP Host 头嗅探。

---

## 🤖 AI 原生端点与多模态协商

`lunar-serve` 被设计为面向 AI Agent 消费的安全、无状态、只读代码接口。

### 1. 画布索引（GET `/`）
原生提供单文件编译后的 `lunar-scope` React 画布。在浏览器中打开 `http://127.0.0.1:8787/`，即可直接加载 3D 拓扑图，无需任何 CORS 摩擦，也无需配置 Nginx 静态托管。

### 2. 文件树与契约摘要（基于 Accept 的多模态响应）
*   **端点**：`GET /:owner/:repo/tree/:branch`
*   **行为**：
    *   **默认（Markdown）**：返回格式优美的 RouteAST 契约摘要、可折叠的活跃待办列表，以及你的本地 VPS 工作区的**递归文件目录树**（已自动排除 `target/`、`node_modules/`、`.pyc` 等构建噪音）。你可以在树代码块内通过 `#` 开头的注释来引导 AI 导航。
    *   **协商（JSON）**：如果请求携带 `Accept: application/json`，则跳过 Markdown 渲染，返回一个高内聚的 JSON 数组，包含所有干净的相对文件路径，便于程序化解析。

### 3. 按需源代码读取（零拷贝原始流）
*   **端点**：`GET /:owner/:repo/raw/:branch/*filepath`（或 `/blob/` 别名）
*   **行为**：安全地返回请求文件的原始文本。它会通过规范路径比对自动检测目录遍历攻击，并验证访问权限。

### 4. AI 交接待办看板
*   **GET `/api/v1/projects/:name/todo`**：获取当前的 AI 任务列表和待合并的契约补丁。
*   **POST `/api/v1/projects/:name/todo`**：更新任务并注册密码学交接数据。
*   **GET `/api/v1/projects/:name/todo/diff`**：返回当前契约与待合并补丁的并排 Markdown 差异对比，供同行评审 AI 模型使用。

---

## 📜 许可证

Apache-2.0
