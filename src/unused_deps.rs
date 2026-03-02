use crate::config::UnusedDepsConfig;
use crate::{Issue, workspace};
use fs_err as fs;
use ignore::WalkBuilder;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;

pub fn check(config: &UnusedDepsConfig) -> Vec<Issue> {
    let root_toml = fs::read_to_string("Cargo.toml").unwrap_or_else(|e| {
        eprintln!("failed to read root Cargo.toml: {e}");
        std::process::exit(1);
    });

    let root: toml::Value = root_toml.parse().unwrap_or_else(|e| {
        eprintln!("failed to parse root Cargo.toml: {e}");
        std::process::exit(1);
    });

    let member_patterns = workspace::extract_member_patterns(&root);
    let member_dirs = workspace::expand_member_patterns(&member_patterns);

    let mut issues = Vec::new();

    for dir in &member_dirs {
        let cargo_path = dir.join("Cargo.toml");
        if !cargo_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&cargo_path).unwrap_or_else(|e| {
            eprintln!("failed to read {}: {e}", cargo_path.display());
            std::process::exit(1);
        });

        let doc: toml::Value = content.parse().unwrap_or_else(|e| {
            eprintln!("failed to parse {}: {e}", cargo_path.display());
            std::process::exit(1);
        });

        // Collect deps: normalized_name -> vec of "[section] original_name"
        let mut deps: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(table) = doc.get(section).and_then(|v| v.as_table()) {
                for name in table.keys() {
                    if config.ignore.iter().any(|i| i == name) {
                        continue;
                    }
                    let normalized = name.replace('-', "_");
                    deps.entry(normalized)
                        .or_default()
                        .push(format!("[{section}] {name}"));
                }
            }
        }

        if deps.is_empty() {
            continue;
        }

        // Walk .rs files and check for usage
        let source_contents = collect_rs_sources(dir);

        let unused: Vec<String> = deps
            .into_iter()
            .filter(|(normalized, _)| {
                let pattern = format!(r"\b{}\b", regex::escape(normalized));
                let re = Regex::new(&pattern).expect("valid regex");
                !source_contents.iter().any(|src| re.is_match(src))
            })
            .flat_map(|(_, labels)| labels)
            .collect();

        if !unused.is_empty() {
            let mut details: Vec<String> = unused;
            details.push(String::new());
            details.push(
                "Note: proc-macro crates and build.rs-generated code may cause false positives."
                    .into(),
            );
            details
                .push("Try removing the flagged deps and run `cargo build --all-targets`.".into());
            details.push(
                "If it breaks, add the dep to [unused-deps] ignore in your workspace-lint config."
                    .into(),
            );

            issues.push(Issue {
                title: format!("Possibly unused deps in {}", cargo_path.display()),
                details,
            });
        }
    }

    issues
}

fn collect_rs_sources(dir: &Path) -> Vec<String> {
    let mut sources = Vec::new();

    for entry in WalkBuilder::new(dir)
        .hidden(false)
        .build()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs")
            && let Ok(content) = fs::read_to_string(path)
        {
            sources.push(content);
        }
    }

    sources
}
