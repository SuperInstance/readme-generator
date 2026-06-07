# readme-generator

> **Parses your Rust crate. Writes your README. Scores what it finds.**

[![crates.io](https://img.shields.io/crates/v/readme-generator.svg)](https://crates.io/crates/readme-generator)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Auto-generates high-quality `README.md` files from Rust crate source code. Uses `syn` to parse your crate's public API, extracts documentation comments, identifies test functions, and produces a structured README with badges, installation instructions, usage examples, and feature lists.

Also includes a **scoring system** that evaluates existing READMEs against best practices — title, description, badges, installation, usage, API documentation, examples, and license.

## Installation

```bash
cargo install readme-generator
```

## Usage

### Generate a README

```bash
# Generate README.md for the current crate
readme-generator generate --crate-path .

# Generate and save to a specific file
readme-generator generate --crate-path ./my-crate --output ./my-crate/README.md
```

### Score an existing README

```bash
# Score README.md against best practices
readme-generator score --crate-path .

# Check multiple crates in a workspace
readme-generator score --crate-path ./workspace-root
```

## How It Works

### Analysis Phase

1. **Parse `Cargo.toml`**: Extract name, version, description, dependencies, license, repository
2. **Parse source files**: Walk `src/` using `syn`, extract:
   - Public functions, structs, enums, traits, type aliases, constants
   - Documentation comments (`///` and `//!`)
   - Test function names and count
3. **Build `CrateInfo`**: Structured representation of the crate's public API

### Generation Phase

From the `CrateInfo`, generate sections:
- **Title + description** (from Cargo.toml or `//!` crate-level docs)
- **Badges** (crates.io, docs.rs, license)
- **Table of Contents** (if enough sections)
- **Features** (from public API)
- **Installation** (`cargo add` command)
- **Usage** (from doc examples)
- **API Reference** (all public items with their docs)
- **License**

### Scoring Phase

Evaluates a README against 8 criteria:

| Section | Max Score | What it checks |
|---------|-----------|----------------|
| Title | 10 | Present, matches crate name |
| Description | 15 | Present, informative |
| Badges | 10 | crates.io, docs.rs, license |
| Installation | 15 | Clear install instructions |
| Usage | 20 | Code examples present |
| API Reference | 15 | Public items documented |
| Examples | 10 | Runnable examples |
| License | 5 | License section present |

## Architecture

```
src/
├── main.rs        # CLI entry point (clap subcommands)
├── lib.rs         # Re-exports all modules
├── analysis.rs    # CrateInfo parser (Cargo.toml + syn AST)
├── generator.rs   # README.md generation from CrateInfo
├── scoring.rs     # README quality scoring (8 criteria)
└── checker.rs     # Batch scoring for workspaces
```

## Part of [SuperInstance](https://github.com/SuperInstance)

Built to ensure every crate in the SuperInstance ecosystem has world-class documentation. The fleet has 300+ crates — manual README writing doesn't scale. This tool automates the baseline and flags repos that need human attention.

## License

MIT © [SuperInstance](https://github.com/SuperInstance)
