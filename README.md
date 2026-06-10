# readme-generator

![Rust](https://img.shields.io/badge/language-Rust%202021-orange)
![License](https://img.shields.io/badge/license-MIT-green)
![SuperInstance](https://img.shields.io/badge/fleet-SuperInstance-orange)

Auto-generate high-quality READMEs from Rust crate source code. Parses your `Cargo.toml`, walks your `src/` tree with `syn`, extracts the public API, and produces a structured README — or scores an existing one.

## Why This Exists

Writing READMEs is mechanical but easy to get wrong. This tool does the mechanical part: reads your crate's actual types, functions, traits, tests, and dependencies, then generates a README that reflects the code as it exists today — not as it existed three months ago when someone last updated the docs manually.

## Installation

```bash
cargo install --path .
```

Requires Rust 2021 edition.

## Usage

### Generate a README

```bash
# Generate README.md for a crate (writes to crate_path/README.md)
readme-generator generate --crate-path ./my-crate

# Show diff without writing
readme-generator generate --crate-path ./my-crate --diff

# Overwrite without prompting
readme-generator generate --crate-path ./my-crate --force

# Custom output path
readme-generator generate --crate-path ./my-crate --output ./docs/README.md
```

### Score an existing README

```bash
# Score README completeness (0-100)
readme-generator check --crate-path ./my-crate

# Score as JSON (for scripting)
readme-generator check --crate-path ./my-crate --json

# Score and show what would change
readme-generator score-diff --crate-path ./my-crate
```

### As a library

```rust
use readme_generator::analysis::analyze_crate;
use readme_generator::generator::generate_readme;
use readme_generator::scoring::score_readme;

let crate_info = analyze_crate(&std::path::Path::new("./my-crate"))?;
let readme = generate_readme(&crate_info)?;

// Score an existing README
let existing = std::fs::read_to_string("./my-crate/README.md")?;
let scores = score_readme(&existing, &crate_info);
println!("Score: {}/{}", scores.total, scores.max_total);
```

## API Reference

### analysis — `src/analysis.rs`

Parses crate structure and extracts metadata.

```rust
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub edition: Option<String>,
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub public_api: PublicApi,
    pub test_functions: Vec<TestFn>,
    pub binary_targets: Vec<String>,
    pub is_workspace: bool,
}

pub struct PublicApi {
    pub functions: Vec<PubItem>,
    pub structs: Vec<PubItem>,
    pub enums: Vec<PubItem>,
    pub traits: Vec<PubItem>,
    pub type_aliases: Vec<PubItem>,
    pub constants: Vec<PubItem>,
}

pub struct PubItem {
    pub name: String,
    pub doc_comments: Vec<String>,
    pub signature: Option<String>,
}

pub fn analyze_crate(path: &Path) -> Result<CrateInfo>
```

Uses `syn` to parse every `.rs` file under `src/`, collects `pub fn`, `pub struct`, `pub enum`, `pub trait`, `type` aliases, and `const` items along with their doc comments and signatures.

### generator — `src/generator.rs`

Produces a complete README string from `CrateInfo`.

```rust
pub fn generate_readme(info: &CrateInfo) -> Result<String>
```

Generated sections: title, description, badges, table of contents, features, installation, usage examples, API reference, testing, dependencies, license.

### scoring — `src/scoring.rs`

Scores an existing README against expected completeness.

```rust
pub struct ScoreData {
    pub total: u64,
    pub max_total: u64,
    pub sections: Vec<SectionScore>,
    pub breakdown: String,
}

pub struct SectionScore {
    pub section: String,   // "Title", "Description", "Installation", etc.
    pub score: u64,
    pub max: u64,
    pub notes: String,
}

pub fn score_readme(readme: &str, info: &CrateInfo) -> ScoreData
```

Scoring breakdown (100 points max):

| Section | Points | What's checked |
|---------|--------|----------------|
| Title | 10 | Present, matches crate name |
| Description | 10 | Present, matches Cargo.toml description |
| Installation | 15 | Contains `cargo add` or `Cargo.toml` instructions |
| Usage | 15 | Has code examples |
| API Reference | 15 | Documents public API items |
| Testing | 10 | Mentions test suite |
| License | 10 | Present and matches Cargo.toml |

### checker — `src/checker.rs`

Thin delegation layer to `scoring::score_readme` for the `check` subcommand.

## Architecture

```
readme-generator/
├── src/
│   ├── main.rs          # CLI (clap): generate, check, score-diff
│   ├── lib.rs           # Re-exports analysis, checker, generator, scoring
│   ├── analysis.rs      # syn-based crate parsing → CrateInfo
│   ├── generator.rs     # CrateInfo → README.md string
│   ├── scoring.rs       # README vs CrateInfo → ScoreData (0-100)
│   └── checker.rs       # CLI bridge to scoring
├── tests/               # Integration tests (assert_cmd, predicates)
├── Cargo.toml
└── CONTRIBUTING.md
```

```
┌─────────────┐     ┌────────────┐     ┌──────────────┐
│  CLI (clap) │────→│  analysis   │────→│  generator   │──→ README.md
│  main.rs    │     │  syn parse  │     │  template    │
└──────┬──────┘     └─────┬──────┘     └──────────────┘
       │                  │
       │                  ↓
       │           ┌──────────────┐
       └──────────→│   scoring    │──→ ScoreData (0-100)
                   │  heuristic   │
                   └──────────────┘
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `syn` (full + parsing + visit) | Parse Rust source into AST |
| `quote` | Token manipulation |
| `clap` (derive) | CLI argument parsing |
| `serde` + `serde_json` | Serialize/deserialize crate info |
| `toml` | Parse `Cargo.toml` |
| `proc-macro2` | Token bridge for syn |
| `anyhow` | Error handling |
| `colored` | Terminal colors |
| `similar` | Diff output for `--diff` mode |

## Related SuperInstance Crates

- **metal-lathe** — Uses readme-generator to evaluate documentation quality across the fleet
- **SuperInstance monorepo** — CI pipeline runs `readme-generator check` on every PR

## License

MIT
