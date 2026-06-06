use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PublicApi {
    pub functions: Vec<PubItem>,
    pub structs: Vec<PubItem>,
    pub enums: Vec<PubItem>,
    pub traits: Vec<PubItem>,
    pub type_aliases: Vec<PubItem>,
    pub constants: Vec<PubItem>,
}

impl PublicApi {
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
            && self.structs.is_empty()
            && self.enums.is_empty()
            && self.traits.is_empty()
            && self.type_aliases.is_empty()
            && self.constants.is_empty()
    }

    pub fn total_items(&self) -> usize {
        self.functions.len()
            + self.structs.len()
            + self.enums.len()
            + self.traits.len()
            + self.type_aliases.len()
            + self.constants.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubItem {
    pub name: String,
    pub doc_comments: Vec<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFn {
    pub name: String,
    pub body_summary: Option<String>,
}

/// Analyze a crate at the given path.
pub fn analyze_crate(path: &Path) -> Result<CrateInfo> {
    let cargo_toml_path = path.join("Cargo.toml");
    anyhow::ensure!(
        cargo_toml_path.exists(),
        "No Cargo.toml found at {}",
        path.display()
    );

    let cargo_content = std::fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Reading {}", cargo_toml_path.display()))?;

    let cargo_toml: toml::Value = cargo_content.parse()
        .with_context(|| format!("Parsing {}", cargo_toml_path.display()))?;

    let package = cargo_toml.get("package");
    let is_workspace = cargo_toml.get("workspace").is_some();

    let default_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let name = package
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(&default_name)
        .to_string();

    let version = package
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();

    let description = package.and_then(|p| p.get("description")).and_then(|v| v.as_str()).map(String::from);
    let license = package.and_then(|p| p.get("license")).and_then(|v| v.as_str()).map(String::from);
    let repository = package.and_then(|p| p.get("repository")).and_then(|v| v.as_str()).map(String::from);
    let edition = package.and_then(|p| p.get("edition")).and_then(|v| v.as_str()).map(String::from);

    let deps = extract_dependency_names(cargo_toml.get("dependencies"));
    let dev_deps = extract_dependency_names(cargo_toml.get("dev-dependencies"));

    let binary_targets = extract_binary_targets(&cargo_toml);

    // Find and parse source files
    let src_dir = path.join("src");
    let mut public_api = PublicApi {
        functions: vec![],
        structs: vec![],
        enums: vec![],
        traits: vec![],
        type_aliases: vec![],
        constants: vec![],
    };
    let mut test_functions = vec![];

    if src_dir.exists() {
        let entries = collect_rust_files(&src_dir);
        for entry in &entries {
            let content = std::fs::read_to_string(entry)
                .with_context(|| format!("Reading {}", entry.display()))?;
            let api = parse_rust_source(&content);
            merge_api(&mut public_api, api);

            let tests = extract_tests(&content);
            test_functions.extend(tests);
        }
    }

    Ok(CrateInfo {
        name,
        version,
        description,
        license,
        repository,
        edition,
        dependencies: deps,
        dev_dependencies: dev_deps,
        public_api,
        test_functions,
        binary_targets,
        is_workspace,
    })
}

fn extract_dependency_names(deps: Option<&toml::Value>) -> Vec<String> {
    let Some(deps) = deps else { return vec![] };
    let Some(table) = deps.as_table() else { return vec![] };
    table.keys().cloned().collect()
}

fn extract_binary_targets(cargo_toml: &toml::Value) -> Vec<String> {
    let mut bins = vec![];
    if let Some(bin_array) = cargo_toml
        .get("bin")
        .and_then(|v| v.as_array())
    {
        for bin in bin_array {
            if let Some(name) = bin.get("name").and_then(|v| v.as_str()) {
                bins.push(name.to_string());
            }
        }
    }
    bins
}

fn collect_rust_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "rs") {
                files.push(path);
            }
        }
    }
    // Also check subdirectories (modules)
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let mod_file = path.with_extension("rs");
                if !mod_file.exists() {
                    // Check for module files inside
                    files.extend(collect_rust_files(&path));
                }
            }
        }
    }
    files
}

/// Parse a Rust source file and extract public API items.
fn parse_rust_source(content: &str) -> PublicApi {
    let mut api = PublicApi {
        functions: vec![],
        structs: vec![],
        enums: vec![],
        traits: vec![],
        type_aliases: vec![],
        constants: vec![],
    };

    let Ok(parsed) = syn::parse_file(content) else {
        return api;
    };

    for item in &parsed.items {
        match item {
            syn::Item::Fn(f) if is_pub(&f.vis) && !is_test_fn(f) => {
                let name = f.sig.ident.to_string();
                let doc_comments = extract_doc_comments(&f.attrs);
                let signature = format_function_signature(f);
                api.functions.push(PubItem { name, doc_comments, signature });
            }
            syn::Item::Struct(s) if is_pub(&s.vis) => {
                let name = s.ident.to_string();
                let doc_comments = extract_doc_comments(&s.attrs);
                api.structs.push(PubItem { name, doc_comments, signature: None });
            }
            syn::Item::Enum(e) if is_pub(&e.vis) => {
                let name = e.ident.to_string();
                let doc_comments = extract_doc_comments(&e.attrs);
                api.enums.push(PubItem { name, doc_comments, signature: None });
            }
            syn::Item::Trait(t) if is_pub(&t.vis) => {
                let name = t.ident.to_string();
                let doc_comments = extract_doc_comments(&t.attrs);
                api.traits.push(PubItem { name, doc_comments, signature: None });
            }
            syn::Item::Type(t) if is_pub(&t.vis) => {
                let name = t.ident.to_string();
                let doc_comments = extract_doc_comments(&t.attrs);
                api.type_aliases.push(PubItem { name, doc_comments, signature: None });
            }
            syn::Item::Const(c) if is_pub(&c.vis) => {
                let name = c.ident.to_string();
                let doc_comments = extract_doc_comments(&c.attrs);
                api.constants.push(PubItem { name, doc_comments, signature: None });
            }
            _ => {}
        }
    }

    api
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn is_test_fn(f: &syn::ItemFn) -> bool {
    f.attrs.iter().any(|attr| {
        attr.path().segments.last().map_or(false, |s| s.ident == "test")
    })
}

fn extract_doc_comments(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().segments.last().map_or(false, |s| s.ident == "doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect()
}

fn format_function_signature(f: &syn::ItemFn) -> Option<String> {
    let name = &f.sig.ident;
    let mut sig = format!("fn {}(", name);

    for (i, input) in f.sig.inputs.iter().enumerate() {
        if i > 0 {
            sig.push_str(", ");
        }
        match input {
            syn::FnArg::Typed(pt) => {
                sig.push_str(&quote::quote!(#pt).to_string().replace(' ', " "));
            }
            syn::FnArg::Receiver(_) => {
                sig.push_str("&self");
            }
        }
    }

    sig.push(')');

    match &f.sig.output {
        syn::ReturnType::Type(_, ty) => {
            sig.push_str(&format!(" -> {}", quote::quote!(#ty)));
        }
        syn::ReturnType::Default => {}
    }

    Some(sig)
}

/// Extract test function names and brief summaries from source.
fn extract_tests(content: &str) -> Vec<TestFn> {
    let Ok(parsed) = syn::parse_file(content) else {
        return vec![];
    };

    let mut tests = vec![];

    for item in &parsed.items {
        if let syn::Item::Fn(f) = item {
            if is_test_fn(f) {
                let name = f.sig.ident.to_string();
                // Take first expression in the block as a summary hint
                let body_summary = extract_first_assert_or_expr(&f.block);
                tests.push(TestFn { name, body_summary });
            }
        }
    }

    tests
}

fn extract_first_assert_or_expr(block: &syn::Block) -> Option<String> {
    let stmt = block.stmts.first()?;
    let expr = match stmt {
        syn::Stmt::Expr(e, _) => e,
        syn::Stmt::Local(_l) => {
            // let ... = ...
            return Some(format!("let binding"));
        }
        _ => return None,
    };
    let text = quote::quote!(#expr).to_string();
    // Truncate for readability
    let truncated = if text.len() > 100 {
        format!("{}...", &text[..100])
    } else {
        text
    };
    Some(truncated)
}

fn merge_api(target: &mut PublicApi, source: PublicApi) {
    target.functions.extend(source.functions);
    target.structs.extend(source.structs);
    target.enums.extend(source.enums);
    target.traits.extend(source.traits);
    target.type_aliases.extend(source.type_aliases);
    target.constants.extend(source.constants);
}
