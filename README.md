# lunar

**Command-line tool for the LunarAST protocol family.**

`lunar` is the core CLI of the LunarAST ecosystem. It performs static extraction, comparison, and synchronization of network interface contracts in multi-language microservice projects.

## Installation

```bash
cargo install lunar
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
```

## Adapters

`lunar` discovers language-specific adapters via `PATH`. Install the adapters you need:

| Language / Framework | Adapter | Repository |
|:---|:---|:---|
| Rust (Axum) | `lunar-extract-rust` | [LunarAST/lunar-extract-rust](https://github.com/LunarAST/lunar-extract-rust) |

To add support for a new language, implement the [LDJSON adapter protocol](https://github.com/LunarAST/RouteAST#31-line-delimited-json-ldjson-output-stream-format).

## Documentation

- [LunarAST Ecosystem Mother Specification](https://github.com/LunarAST/.github/blob/main/docs/ecosystem-whitepaper-v1.0.md)
- [RouteAST Sub-Protocol](https://github.com/LunarAST/RouteAST)

## License

Apache-2.0
