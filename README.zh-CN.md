# lunar
**LunarAST 命令行工具 — 接口契约静态提取、差异比对与 GitOps 协同同步**

`lunar` 是 LunarAST 生态的核心命令行引擎，基于 Rust 开发，自动化完成项目初始化、代码抽象语法树静态扫描、契约差异诊断，以及**零信任交互式AI补丁合并**；全程无需记忆命令、无需手动复制粘贴文件。

---

## 🏛️ 解耦式工作区架构
遵循「轻入口、重内核库」设计思想，`lunar` 采用高度解耦、模块化的 Cargo 工作区结构：
```
lunar/
├── Cargo.toml
└── src/
    ├── lib.rs            # 核心库模块声明
    ├── main.rs           # 极简程序入口（仅70行路由分发代码）
    ├── commands/         # 子命令执行模块（边界完全隔离）
    │   ├── mod.rs        # 命令导出器
    │   ├── scan.rs       # 触发静态代码分析
    │   ├── diff.rs       # 接口契约差异诊断
    │   ├── sync.rs       # 本地人工合并逻辑
    │   ├── pull.rs       # 非交互式安全TCP拉取与合并待办补丁
    │   ├── serve.rs      # 启动本地分发服务进程
    │   ├── interactive.rs   # 状态驱动交互式菜单
    │   ├── setup_totp.rs    # TOTP密钥生成并输出二维码
    │   ├── visibility.rs    # 项目可见性管理器（锁定/解锁/切换）
    │   ├── sync_visibility.rs # 通过GitHub API同步项目公开状态
    │   └── sync_repos.rs      # 读取Git元数据生成repos.json
    ├── map.rs            # 全局拓扑图生成、自动路径扫描
    ├── cleanup.rs        # 本地缓存与归档清理工具
    ├── doctor.rs         # 运行环境与一致性校验
    ├── keygen.rs         # Ed25519加密密钥对生成器
    ├── guide.rs          # 自适应状态交互式菜单工具
    └── uploader.rs       # S3/R2 对象存储上传客户端
```

---

## ⚡ 快速上手（开箱即用交互体验）
### 1. 安装
进入 `/opt/LunarAST/lunar` 目录，编译并全局安装：
```bash
cargo install --path .
```

### 2. 无参数交互式菜单
服务器/本地终端**任意目录**直接输入 `lunar`，无需附加参数：
```bash
lunar
```

#### 启动后信息面板展示内容：
* **当前服务端口**：读取配置或环境变量 `LUNAR_SERVE_PORT`，默认 8787，启动时可二次确认
* **对外域名**：自动解析服务公网地址（示例：`shturl.cc/GCplkeKDVcSMdgN4kC5wvGBL4zvqOMFnYRr4wIA9prZGTLgbC`，无配置则回退本地地址）
* **工作区根目录**：当前终端所在物理路径
* **TOTP状态**：展示双因素认证密钥是否已配置

#### 🔔 智能上下文自动检测：
程序启动时自动读取全局 `lunar-map.json`，扫描所有项目待办补丁列表。若检测到待合并AI补丁，直接跳过菜单，弹出醒目提示：
```
🌙 LunarAST — 生态接口契约管控系统

  🔔 检测到项目「cellrix」存在待合并AI补丁（已通过网页端完成初审）
     → 自动合并并刷新拓扑图？[Y/n]: 
```
直接回车即可执行：工具自动拉取补丁、校验Ed25519加密签名、合并至本地 `interfaces.yml`、自动备份旧配置、标记任务完成、重新生成全局拓扑文件。

前端 `lunar-scope` 可视化画布将在0.5秒内实时刷新！

---

## 🆕 v3.0 安全与自动化新增功能
### 1. 身份认证配置（setup-totp）
```bash
lunar setup-totp
```
生成 Base32 编码 TOTP 密钥，输出 ASCII 二维码，可绑定谷歌验证器、Authy 等App；密钥文件存储在 `.lunar/totp.secret`，文件权限设为 600。若已存在密钥，更换密钥前需验证当前有效验证码。

### 2. 项目可见性管理器（visibility）
```bash
lunar visibility
```
交互式子菜单支持：
- **全部锁定**：所有项目设为私有
- **全部解锁**：所有项目设为公开
- **单独切换**：修改指定项目的公开/私有状态
- **同步GitHub状态**：调用GitHub API同步仓库可见性（需配置 `GITHUB_TOKEN`）

### 3. 仓库元数据同步（sync-repos）
```bash
lunar sync-repos
```
读取 `lunar-map.json` 获取全部项目路径，从每个项目本地 `.git` 配置中提取 **GitHub 所有者、仓库名、当前分支**，写入 `repos.json`；生成LCT加密令牌时会自动修正分支信息。

### 4. 服务进程管理（交互菜单内操作）
进入交互菜单且扫描数据就绪后：
- 按 `5` 启动 `lunar-serve` 服务
- 按 `8` 停止运行中的服务（通过PID文件发送SIGTERM优雅关闭）
- 按 `9` 重启服务

无需手动执行 `pkill -f lunar-serve` 杀进程。

### 5. 新增环境变量
- `GITHUB_TOKEN` — GitHub个人访问令牌，用于同步仓库可见性（可选）
- `LUNAR_SERVE_PORT` — 自定义分发服务端口（默认8787）
- `LUNAR_SERVE_DOMAIN` — 生成令牌链接使用的公网域名（示例：`shturl.cc/GCplkeKDVcSMdgN4kC5wvGBL4zvqOMFnYRr4wIA9prZGTLgbC`）

---

## 🔒 自动清理工具
防止本地归档、审计日志无限占用磁盘，按需执行清理命令：
```bash
lunar cleanup --archive --days 30
```
自动清理超过指定天数（默认30天）的按日分割JSONL审计日志与任务归档文件，可控磁盘占用。

---

## 📜 开源协议
Apache-2.0
