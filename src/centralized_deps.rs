use crate::{Issue, workspace};
use fs_err as fs;
use std::collections::BTreeSet;

pub fn check() -> Vec<Issue> {
    let root_toml = fs::read_to_string("Cargo.toml").unwrap_or_else(|e| {
        eprintln!("failed to read root Cargo.toml: {e}");
        std::process::exit(1);
    });

    let root: toml::Value = root_toml.parse().unwrap_or_else(|e| {
        eprintln!("failed to parse root Cargo.toml: {e}");
        std::process::exit(1);
    });

    let workspace_dep_names = extract_workspace_dep_names(&root);
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

        let mut crate_errors: Vec<String> = Vec::new();

        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc.get(section).and_then(|v| v.as_table()) {
                for (name, value) in deps {
                    if let Some(msg) = check_dep(name, value, section, &workspace_dep_names) {
                        crate_errors.push(msg);
                    }
                }
            }
        }

        if !crate_errors.is_empty() {
            issues.push(Issue {
                title: format!("Fix workspace deps in {}", cargo_path.display()),
                details: crate_errors,
            });
        }
    }

    issues
}

fn check_dep(
    name: &str,
    value: &toml::Value,
    section: &str,
    workspace_deps: &BTreeSet<String>,
) -> Option<String> {
    match value {
        // Simple string version: dep = "1.0"
        toml::Value::String(version) => {
            if workspace_deps.contains(name) {
                Some(format!(
                    "[{section}] {name}: has own version \"{version}\" — use {{ workspace = true }} instead"
                ))
            } else {
                Some(format!(
                    "[{section}] {name}: version \"{version}\" not in [workspace.dependencies] — add it there and use {{ workspace = true }}"
                ))
            }
        }
        // Table: dep = { ... }
        toml::Value::Table(table) => {
            // workspace = true → OK
            if table
                .get("workspace")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return None;
            }

            // path dependency without workspace → skip (local override)
            if table.contains_key("path") {
                return None;
            }

            // Has explicit version
            if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
                if workspace_deps.contains(name) {
                    Some(format!(
                        "[{section}] {name}: has own version \"{version}\" — use {{ workspace = true }} instead"
                    ))
                } else {
                    Some(format!(
                        "[{section}] {name}: version \"{version}\" not in [workspace.dependencies] — add it there and use {{ workspace = true }}"
                    ))
                }
            } else if table.contains_key("git") {
                // git dependency without workspace → check if in workspace deps
                if workspace_deps.contains(name) {
                    Some(format!(
                        "[{section}] {name}: has own git source — use {{ workspace = true }} instead"
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_workspace_dep_names(root: &toml::Value) -> BTreeSet<String> {
    root.get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}
