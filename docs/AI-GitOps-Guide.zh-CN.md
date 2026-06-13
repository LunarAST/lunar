# 🌙 LunarAST AI-GitOps 极简操作与协同规范手册

本手册指导人类开发人员与**自主 AI 代理（Autonomous AI Agent）**，如何基于 LunarAST 生态在零信任环境下，开展零命令记忆、零人工粘贴、安全、合规且支持原子化回滚的 AI 协同契约开发。

---

## 🏛️ 1. 三级递进真理源架构

LunarAST 引入以下**三级递进真理源**来对齐微服务群的网络契约事实，不引入任何运行期监控与性能损耗：

1. **第一级：物理事实（`.interfaces-autogen.json`）**：由适配器从语法树中自动提取的 API 原始事实，此文件必须加入项目的 `.gitignore`。
2. **第二级：意图覆盖层（`interfaces.yml`）**：由人类版本控制（Git）下的手动覆盖定义。对齐引擎在合并时执行 **“字段级部分覆盖（Partial Field Override）”** 算法，自动合并 AI 数据，100% 保留人类写过的任何自定义注释和属性。
3. **第三级：末端逃生舱**：在源码行上方通过魔法单行注释（`// lunar:consume`）声明由于动态求值导致无法被 AST 静态分析捕获的边缘依赖。

---

## 🧭 2. 人类开发者本地操作流（极简 CLI 体验）

为了消灭“记不住命令”的老旧痛点，`lunar` CLI 引入了高度直觉的参数看板和引导菜单。您在 VPS 或本地电脑的终端上只需运行：

```bash
lunar
```

不带任何参数启动 `lunar` 会直接进入**交互引导菜单**。它会在最顶部为您高亮展示当前的**“现役物理指标看板”**：
* **Active Port**：常驻的只读服务端端口（默认 `8787`，支持环境变量 `LUNAR_SERVE_PORT` 覆盖与启动确认）。
* **Active Domain**：现役公网主权域名（默认 `https://lunar.aifify.com`）。
* **Workspace Root**：当前工作区的绝对物理路径。

### 核心菜单操作：
*   `[1] Scan project`：极速扫描并重新生成本地事实。
*   `[5] Launch serving daemon`：自动在后台拉起解耦后的 `lunar-serve` 服务器。
*   `[6] Generate topology`：重构并编译全局契约拓扑。它会自动在 VPS 硬盘上搜索子项目，将自动捕获出的**物理绝对路径（如 `/opt/cellrix`）** 注入到地图中，服务端以此实现零人工配置源码定位。

---

## 🤖 3. 引导 AI 睁眼（大小写自适应归一化）

当您在网页端或 Chat 界面中与任何外部 AI 协同开发时，您只需向其投喂该项目的 `/tree` 契约地址：
👉 `https://lunar.aifify.com/Jasonmilk/cellrix/tree/rs2`

网关在接收到请求后，会自动执行**小写归一化（Lowercase Normalisation）哈希映射**（将 `Jasonmilk/cellrix/rs2` 转换对齐），彻底抹平大小写命名差异导致的分发断层。

---

## 🛠️ 4. AI 在开发过程中的“按需获取与自我生长”

AI 代理在读到 URL 后，将自动睁开双眼，并按照以下 GitOps 规范执行自组织循环：

### 第一步：读取顶端引导词，看清目录树大纲
AI 发起 GET 请求访问 `/tree/rs2`。`lunar-serve` 会首先检测并动态加载项目下的 `.lunar/ai-instruction.md` 提示词 [1.2]（如无，则降级使用默认模板，彻底避免硬编码），将其拼装在输出页面的**最顶端第一行**。
*   **AI 学习准则**：AI 会直接读到文件树顶部由 `#` 开头的物理注释指引，瞬间无感学会如何读取 README、如何发起 `/raw` 原始代码拉取、以及如何更新 todo 待办。
*   **AI 观察目录**：AI 浏览页面底部排好版、并且自动过滤了 `target`、`.pyc` 等临时/构建垃圾的极简目录树，瞬间在脑中形成工作区地貌。

### 第二步：精准按需获取代码事实
AI 不会盲猜。它会根据目录树，向指定文件发起精准的单点 raw 拉取：
👉 `GET https://lunar.aifify.com/Jasonmilk/cellrix/raw/rs2/crates/cellrix-core/src/lib.rs`

### 第三步：契约自我生长与任务交接
AI 在重构完代码后，发现契约缺失。它自己逆向写好符合规范的 YAML 补丁，并使用它本地的 **AI 专属私钥**，对补丁内容进行 **Ed25519 数字签名**，最后 POST 推送上墙：
👉 `POST https://lunar.aifify.com/api/v1/projects/cellrix/todo`

---

## 🔒 5. 人类双重门禁审核与一键对齐（Zero-Friction CD）

为防范 AI 的胡言乱语与恶意注入，AI 绝对没有写权限。您在终端一秒完成终审与对齐：

### 第一步：交叉模型同行审计（AI-on-AI 审计）
您将看板生成的 comparative Markdown 差异地址，直接丢给另一个 AI 审计师（如 Claude 3.5）：
👉 `https://lunar.aifify.com/api/v1/projects/cellrix/todo/diff`
Claude 在云端直读此对比图，帮您把关是否有 `MethodMismatch` 或 `Orphaned` 风险。

### 第二步：人类一键回车大对齐
在 VPS 的任意绝对路径下，您直接打一个：
```bash
lunar
```
*   **自愈探针激活**：CLI 自动探测全局地图并发现 `cellrix` 存在待合并的 AI 补丁。主菜单不会加载，而是**直接在顶部弹出高亮提示**：
    `🔔 Detected pending AI patch for project 'cellrix' (already reviewed via web)`
    `→ Auto-merge and refresh map? [Y/n]: `
*   **操作**：您直接按下 **回车（Enter）**！
    CLI 自动拉取补丁、**执行 Ed25519 签名验证防伪**、原子化并入本地 `interfaces.yml`、写入旧文件备份、自动把看板上的任务标记为 `completed` 已完成、最后自动在后台编译地图！
*   **结果**：刷新 `https://lunar.aifify.com/`，`cellrix` 节点的 Exposed 接口端口**瞬间优雅地点亮在大画布上**！

---

## 🛡️ 6. 零信任原子化回滚

如果您在合并后，发现 AI 的补丁有缺陷，想要撤销，可在项目目录下 1 毫秒瞬间物理还原：
*   **第一级（系统备份还原）**：
    `cp $(ls -t .lunar/.backup/interfaces.yml.bak.* | head -n 1) .lunar/interfaces.yml`
*   **第二级（Git 原生恢复）**：
    `git checkout .lunar/interfaces.yml`
