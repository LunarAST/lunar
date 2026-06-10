# lunar
**LunarAST 协议体系命令行工具**

`lunar` 是 LunarAST 生态的核心命令行工具，针对多语言微服务项目的接口契约，提供**静态提取、差异对比、配置同步、多项目拓扑映射**与**健康诊断**等全套能力。

## 安装
```bash
cargo install lunar
```

也可通过源码编译安装：
```bash
git clone https://github.com/LunarAST/lunar.git
cd lunar
cargo build --release
```

## 快速上手
```bash
# 初始化项目配置
lunar init

# 扫描当前项目，提取接口契约
lunar scan

# 将当前接口与上一版快照进行对比
lunar diff

# 应用变更至意图覆盖配置（自动备份原有文件）
lunar sync --apply

# 执行生态一致性健康检测
lunar doctor

# 清理本地缓存文件
lunar cleanup --all

# 生成多项目全局拓扑文件
lunar map -o lunar-map.json
```

## 命令说明

| 命令 | 功能描述 |
|:---|:---|
| `lunar init` | 若配置文件不存在，则初始化 `.lunar/interfaces.yml`。 |
| `lunar scan` | 通过对应语言适配器静态提取路由信息。Rust 项目支持 `--rustdoc` 模式（需夜间版工具链）以获得最高解析精度，结果写入 `.lunar/.interfaces-autogen.json`。 |
| `lunar diff` | 对比当前路由与上一次扫描结果，展示新增、删除、修改的接口（包含请求方法、参数名称变更）。 |
| `lunar sync --dry-run` | 预览即将写入 `.lunar/interfaces.yml` 的所有变更。 |
| `lunar sync --apply` | 备份现有 `interfaces.yml`；扫描 `.lunar/suggestions/*.yaml` 目录下由人工或 AI 生成的待处理补丁；**展示差异预览并等待用户确认**；以字段级增量合并方式更新配置；写入新版 `interfaces.yml`，并将已处理补丁归档为 `.lunar/suggestions/*.yaml.applied`。 |
| `lunar patch` | 从文件或标准输入载入 YAML 契约补丁并应用。支持文件模式（`lunar patch path/to/file.yaml`）与管道模式（`cat patch.yaml \| lunar patch`）；若存在 `repos.json`，会校验目标项目合法性，合并前需人工确认。 |
| `lunar map` | 生成生态拓扑图。指定 `--config` 参数则读取配置文件；不指定配置时，自动遍历 `LUNAR_PROJECTS_DIR` 目录（默认 `/opt/`）下所有项目。支持 `--upload` 与 `--output` 参数。 |
| `lunar doctor` | 执行生态健康检测：校验适配器是否存在、扫描数据合法性、缓存文件完整性。<br>退出码说明：`0`=环境正常、`1`=环境异常、`2`=数据异常。 |
| `lunar cleanup --all` | 清理本地扫描缓存文件 `.lunar/.interfaces-autogen.json`。默认需要交互式确认，添加 `--yes` 参数可跳过确认直接执行。 |

## 环境变量
- `LUNAR_PROJECTS_DIR` — 未指定 `--config` 时，`lunar map` 检索项目的根目录，默认值为 `/opt`。

## 适配器
`lunar` 会通过系统环境变量 `PATH` 自动发现各语言专属适配器，请按需安装对应组件：

| 语言 / 框架 | 适配器名称 | 项目仓库 |
|:---|:---|:---|
| Rust (Axum) | `lunar-extract-rust` | [LunarAST/lunar-extract-rust](https://github.com/LunarAST/lunar-extract-rust) |

如需新增其他语言支持，请按照 [LDJSON 适配器协议](https://github.com/LunarAST/RouteAST#31-line-delimited-json-ldjson-output-stream-format) 开发对应适配器。

## 生态镜像服务
若希望通过类 GitHub 格式的镜像域名访问项目（示例：`https://name.your-domain.com/your-owner/your-repo/tree/main`），请参考 [lunar-serve](https://github.com/LunarAST/lunar-serve) 文档完成部署与项目注册。

## 相关文档
- [LunarAST 生态总规范](https://github.com/LunarAST/.github/blob/main/docs/ecosystem-whitepaper-v1.0.md)
- [RouteAST 子协议](https://github.com/LunarAST/RouteAST)
- [lunar-scope 可视化画布](https://github.com/LunarAST/lunar-scope)
- [lunar-serve 数据分发层](https://github.com/LunarAST/lunar-serve)

## 开源许可证
Apache-2.0
