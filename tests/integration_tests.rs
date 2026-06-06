#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;
    use std::path::Path;

    // Helper to create a minimal crate directory
    fn create_test_crate(name: &str, cargo_toml: &str, lib_rs: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        let base = dir.path();
        fs::write(base.join("Cargo.toml"), cargo_toml).unwrap();
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("src/lib.rs"), lib_rs).unwrap();
        dir
    }

    // ── Analysis Tests ──

    #[test]
    fn test_parses_cargo_toml_name_and_version() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "my-awesome-crate"
version = "1.2.3"
edition = "2021"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert_eq!(info.name, "my-awesome-crate");
        assert_eq!(info.version, "1.2.3");
    }

    #[test]
    fn test_parses_description_and_license() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"
description = "A cool test crate"
license = "MIT"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert_eq!(info.description.as_deref(), Some("A cool test crate"));
        assert_eq!(info.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn test_parses_dependencies() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1"
tokio = { version = "1", features = ["full"] }
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert!(info.dependencies.contains(&"serde".to_string()));
        assert!(info.dependencies.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_parses_dev_dependencies() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"

[dev-dependencies]
tempfile = "3"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert!(info.dev_dependencies.contains(&"tempfile".to_string()));
    }

    #[test]
    fn test_extracts_public_structs() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"
"#,
            r#"/// A user record
pub struct User {
    pub name: String,
}

struct InternalStruct {}
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert_eq!(info.public_api.structs.len(), 1);
        assert_eq!(info.public_api.structs[0].name, "User");
        assert_eq!(info.public_api.structs[0].doc_comments[0], "A user record");
    }

    #[test]
    fn test_extracts_public_enums() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"
"#,
            r#"/// The status of an order
pub enum Status {
    Active,
    Inactive,
}

enum Hidden {}
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert_eq!(info.public_api.enums.len(), 1);
        assert_eq!(info.public_api.enums[0].name, "Status");
    }

    #[test]
    fn test_extracts_public_functions() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"
"#,
            r#"/// Creates a new user
pub fn create_user(name: &str) -> User {
    User { name: name.to_string() }
}

fn internal_helper() {}
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert_eq!(info.public_api.functions.len(), 1);
        assert_eq!(info.public_api.functions[0].name, "create_user");
        assert!(info.public_api.functions[0].signature.is_some());
    }

    #[test]
    fn test_extracts_public_traits() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"
"#,
            r#"/// A trait for processing
pub trait Processor {
    fn process(&self, input: &str) -> String;
}

trait Secret {}
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert_eq!(info.public_api.traits.len(), 1);
        assert_eq!(info.public_api.traits[0].name, "Processor");
    }

    #[test]
    fn test_extracts_test_functions() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"
"#,
            r#"#[test]
fn test_user_creation() {
    assert_eq!(1 + 1, 2);
}

#[test]
fn test_validation_works() {
    assert!(true);
}

pub fn not_a_test() {}
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert_eq!(info.test_functions.len(), 2);
        assert_eq!(info.test_functions[0].name, "test_user_creation");
        assert_eq!(info.test_functions[1].name, "test_validation_works");
    }

    #[test]
    fn test_extracts_binary_targets() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "test"
version = "0.1.0"

[[bin]]
name = "my-cli"
path = "src/main.rs"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert!(info.binary_targets.contains(&"my-cli".to_string()));
    }

    #[test]
    fn test_detects_workspace() {
        let dir = create_test_crate(
            "test-crate",
            r#"[workspace]
members = ["crates/a", "crates/b"]

[package]
name = "workspace-root"
version = "0.1.0"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        assert!(info.is_workspace);
    }

    #[test]
    fn test_fails_on_missing_cargo_toml() {
        let dir = TempDir::new().unwrap();
        let result = readme_generator::analysis::analyze_crate(dir.path());
        assert!(result.is_err());
    }

    // ── Generator Tests ──

    #[test]
    fn test_generates_readme_with_title() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "my-crate"
version = "0.2.0"
description = "A test crate"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        let readme = readme_generator::generator::generate_readme(&info).unwrap();
        assert!(readme.starts_with("# my-crate"));
        assert!(readme.contains("A test crate"));
    }

    #[test]
    fn test_generates_installation_section() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "my-crate"
version = "0.2.0"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        let readme = readme_generator::generator::generate_readme(&info).unwrap();
        assert!(readme.contains("## Installation"));
        assert!(readme.contains("Cargo.toml"));
    }

    #[test]
    fn test_generates_testing_section_when_tests_exist() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "my-crate"
version = "0.2.0"
"#,
            r#"#[test]
fn test_basic() { assert!(true); }
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        let readme = readme_generator::generator::generate_readme(&info).unwrap();
        assert!(readme.contains("## Testing"));
        assert!(readme.contains("test_basic"));
        assert!(readme.contains("cargo test"));
    }

    #[test]
    fn test_generates_api_reference_for_public_items() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "my-crate"
version = "0.2.0"
"#,
            r#"pub struct Config { pub key: String }
pub enum Mode { Fast, Slow }
pub fn run() {}
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        let readme = readme_generator::generator::generate_readme(&info).unwrap();
        assert!(readme.contains("## API Reference"));
        assert!(readme.contains("Config"));
        assert!(readme.contains("Mode"));
        assert!(readme.contains("run"));
    }

    // ── Scoring Tests ──

    #[test]
    fn test_scores_empty_readme_low() {
        use readme_generator::analysis::CrateInfo;
        let info = CrateInfo {
            name: "test".into(),
            version: "0.1.0".into(),
            description: Some("test".into()),
            license: Some("MIT".into()),
            repository: None,
            edition: Some("2021".into()),
            dependencies: vec!["serde".into()],
            dev_dependencies: vec![],
            public_api: Default::default(),
            test_functions: vec![],
            binary_targets: vec![],
            is_workspace: false,
        };
        let score = readme_generator::scoring::score_readme("", &info);
        assert!(score.total < 30, "Empty README should score below 30, got {}", score.total);
    }

    #[test]
    fn test_scores_generated_readme_high() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "scored-crate"
version = "0.3.0"
description = "A crate for scoring"
license = "MIT"
"#,
            r#"/// A struct
pub struct Scorer { pub value: i32 }
/// Does scoring
pub fn score() -> i32 { 42 }
#[test]
fn test_scoring() { assert_eq!(1, 1); }
"#,
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        let readme = readme_generator::generator::generate_readme(&info).unwrap();
        let score = readme_generator::scoring::score_readme(&readme, &info);
        assert!(score.total >= 70, "Generated README should score >= 70, got {}", score.total);
    }

    #[test]
    fn test_score_json_output() {
        let dir = create_test_crate(
            "test-crate",
            r#"[package]
name = "json-test"
version = "0.1.0"
"#,
            "",
        );
        let info = readme_generator::analysis::analyze_crate(dir.path()).unwrap();
        let readme = readme_generator::generator::generate_readme(&info).unwrap();
        let score = readme_generator::scoring::score_readme(&readme, &info);
        let json = serde_json::to_string(&score).unwrap();
        assert!(json.contains("\"total\""));
        assert!(json.contains("\"sections\""));
    }
}
