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
