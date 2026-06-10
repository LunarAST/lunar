# lunar

**Command-line tool for the LunarAST protocol family.**

`lunar` is the core CLI of the LunarAST ecosystem. It performs static extraction, comparison, synchronization, multi-project topology mapping, and health diagnostics for network interface contracts in multi-language microservice projects.

## Installation

```bash
cargo install lunar
```

Or build from source:

```bash
git clone https://github.com/LunarAST/lunar.git
cd lunar
cargo build --release
```

## Quick Start

```bash
# Initialize a new project
lunar init

# Scan current project and extract interface contracts
lunar scan

# Compare current contracts with last saved version
lunar diff

# Apply changes to the intent overlay (with automatic backup)
lunar sync --apply

# Run ecosystem consistency diagnostics
lunar doctor

# Remove local cache files
lunar cleanup --all

# Generate a multi-project topology map
lunar map -o lunar-map.json
```

## Commands

| Command | Description |
|:---|:---|
| `lunar init` | Initialize `.lunar/interfaces.yml` if it does not exist. |
| `lunar scan` | Statically extract routes via language adapters. For Rust projects, supports `--rustdoc` mode (requires nightly) for maximum accuracy. Writes `.lunar/.interfaces-autogen.json`. |
| `lunar diff` | Compare current routes with the last scan, showing added, removed, and modified interfaces (method changes, parameter name changes). |
| `lunar sync --dry-run` | Preview changes that would be written to `.lunar/interfaces.yml`. |
| `lunar sync --apply` | Backup existing `interfaces.yml`, scan `.lunar/suggestions/*.yaml` for pending AI or human-authored patches, **show a diff preview and prompt for confirmation**, perform field-level partial merge (⊕), write the updated `interfaces.yml`, and move processed suggestions to `.lunar/suggestions/*.yaml.applied`. |
| `lunar patch` | Apply a YAML contract patch from file or stdin. Supports file mode (`lunar patch path/to/file.yaml`) and pipe mode (`cat patch.yaml \| lunar patch`). Validates `targetProject` against `repos.json` if present. Prompts for confirmation before merging. |
| `lunar map` | Aggregate `actual.json` files from multiple projects into a global `lunar-map.json` (with per-port alignment data). |
| `lunar doctor` | Run ecosystem health checks: adapter presence, scan data validity, cache integrity. Returns exit code 0 (healthy), 1 (environment error), or 2 (data error). |
| `lunar cleanup --all` | Remove local scan cache files (`.lunar/.interfaces-autogen.json`). Requires interactive confirmation unless `--yes` is passed. |

## Adapters

`lunar` discovers language-specific adapters via `PATH`. Install the adapters you need:

| Language / Framework | Adapter | Repository |
|:---|:---|:---|
| Rust (Axum) | `lunar-extract-rust` | [LunarAST/lunar-extract-rust](https://github.com/LunarAST/lunar-extract-rust) |

To add support for a new language, implement the [LDJSON adapter protocol](https://github.com/LunarAST/RouteAST#31-line-delimited-json-ldjson-output-stream-format).

## Ecosystem Mirror

To make your projects accessible via GitHub-style mirror URLs (e.g., `https://lunar.aifify.com/Jasonmilk/MyRepo/tree/main`), see [lunar-serve](https://github.com/LunarAST/lunar-serve) for setup and project registration.

## Documentation

- [LunarAST Ecosystem Mother Specification](https://github.com/LunarAST/.github/blob/main/docs/ecosystem-whitepaper-v1.0.md)
- [RouteAST Sub-Protocol](https://github.com/LunarAST/RouteAST)
- [lunar-scope Visualization Canvas](https://github.com/LunarAST/lunar-scope)
- [lunar-serve Distribution Layer](https://github.com/LunarAST/lunar-serve)

## License

Apache-2.0
