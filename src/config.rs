use fs_err as fs;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub checks: Checks,
    #[serde(default, rename = "file-size")]
    pub file_size: Option<FileSizeConfig>,
    #[serde(default, rename = "crate-size")]
    pub crate_size: Option<CrateSizeConfig>,
    #[serde(default)]
    pub freshness: Option<FreshnessConfig>,
}

#[derive(Deserialize, Default)]
pub struct Checks {
    #[serde(default, rename = "mise-tasks")]
    pub mise_tasks: bool,
    #[serde(default, rename = "wasm-bindgen-version")]
    pub wasm_bindgen_version: bool,
    #[serde(default, rename = "workspace-deps")]
    pub workspace_deps: bool,
}

#[derive(Deserialize)]
pub struct FileSizeConfig {
    pub rules: Vec<FileSizeRule>,
}

#[derive(Deserialize)]
pub struct FileSizeRule {
    pub glob: String,
    #[serde(rename = "max-code-lines")]
    pub max_code_lines: usize,
}

#[derive(Deserialize)]
pub struct CrateSizeConfig {
    pub rules: Vec<CrateSizeRule>,
}

#[derive(Deserialize)]
pub struct CrateSizeRule {
    pub glob: String,
    #[serde(rename = "max-code-lines")]
    pub max_code_lines: usize,
    pub include: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct FreshnessConfig {
    pub rules: Vec<FreshnessRule>,
}

#[derive(Deserialize)]
pub struct FreshnessRule {
    pub glob: String,
    #[serde(rename = "depends-on")]
    pub depends_on: String,
}

const STANDALONE_FILE: &str = ".workspace-lint.toml";

pub fn load() -> Config {
    let standalone_exists = Path::new(STANDALONE_FILE).exists();
    let cargo_metadata = read_cargo_metadata();

    match (standalone_exists, cargo_metadata) {
        (true, Some(_)) => {
            eprintln!(
                "error: found both {STANDALONE_FILE} and [workspace.metadata.workspace-lint] in Cargo.toml — use only one"
            );
            std::process::exit(1);
        }
        (false, None) => {
            eprintln!(
                "error: no configuration found — create {STANDALONE_FILE} or add [workspace.metadata.workspace-lint] to Cargo.toml"
            );
            std::process::exit(1);
        }
        (true, None) => {
            let content = fs::read_to_string(STANDALONE_FILE).unwrap_or_else(|e| {
                eprintln!("failed to read {STANDALONE_FILE}: {e}");
                std::process::exit(1);
            });
            parse_config(&content, STANDALONE_FILE)
        }
        (false, Some(raw)) => parse_config(&raw, "Cargo.toml [workspace.metadata.workspace-lint]"),
    }
}

fn parse_config(toml_str: &str, source: &str) -> Config {
    toml::from_str(toml_str).unwrap_or_else(|e| {
        eprintln!("failed to parse config from {source}: {e}");
        std::process::exit(1);
    })
}

/// Read the `[workspace.metadata.workspace-lint]` section from Cargo.toml,
/// re-serialized as a standalone TOML string so we can deserialize it into Config.
fn read_cargo_metadata() -> Option<String> {
    let content = fs::read_to_string("Cargo.toml").ok()?;
    let doc: toml::Value = content.parse().ok()?;
    let section = doc
        .get("workspace")?
        .get("metadata")?
        .get("workspace-lint")?;
    Some(toml::to_string(section).expect("failed to re-serialize workspace-lint metadata"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_config() {
        let toml = r#"
[checks]
mise-tasks = true
wasm-bindgen-version = true
workspace-deps = true

[[file-size.rules]]
glob = "**/*.rs"
max-code-lines = 500

[[file-size.rules]]
glob = "**/*.ts"
max-code-lines = 300

[[crate-size.rules]]
glob = "crates/*"
max-code-lines = 5000
include = ["*.rs"]

[[crate-size.rules]]
glob = "crates/web-*"
max-code-lines = 8000
include = ["*.rs", "*.ts"]

[[freshness.rules]]
glob = "**/CLAUDE.md"
depends-on = "**/*.rs"
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.checks.mise_tasks);
        assert!(config.checks.wasm_bindgen_version);
        assert!(config.checks.workspace_deps);

        let fs_rules = config.file_size.unwrap().rules;
        assert_eq!(fs_rules.len(), 2);
        assert_eq!(fs_rules[0].glob, "**/*.rs");
        assert_eq!(fs_rules[0].max_code_lines, 500);
        assert_eq!(fs_rules[1].glob, "**/*.ts");
        assert_eq!(fs_rules[1].max_code_lines, 300);

        let cs_rules = config.crate_size.unwrap().rules;
        assert_eq!(cs_rules.len(), 2);
        assert_eq!(cs_rules[0].glob, "crates/*");
        assert_eq!(cs_rules[0].max_code_lines, 5000);
        assert_eq!(cs_rules[0].include.as_ref().unwrap(), &["*.rs"]);
        assert_eq!(cs_rules[1].include.as_ref().unwrap(), &["*.rs", "*.ts"]);

        let fr_rules = config.freshness.unwrap().rules;
        assert_eq!(fr_rules.len(), 1);
        assert_eq!(fr_rules[0].glob, "**/CLAUDE.md");
        assert_eq!(fr_rules[0].depends_on, "**/*.rs");
    }

    #[test]
    fn parse_empty_config_defaults_all_disabled() {
        let config: Config = toml::from_str("").unwrap();
        assert!(!config.checks.mise_tasks);
        assert!(!config.checks.wasm_bindgen_version);
        assert!(!config.checks.workspace_deps);
        assert!(config.file_size.is_none());
        assert!(config.crate_size.is_none());
        assert!(config.freshness.is_none());
    }

    #[test]
    fn parse_partial_checks() {
        let toml = r#"
[checks]
mise-tasks = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.checks.mise_tasks);
        assert!(!config.checks.wasm_bindgen_version);
        assert!(!config.checks.workspace_deps);
    }

    #[test]
    fn parse_only_file_size_rules() {
        let toml = r#"
[[file-size.rules]]
glob = "**/*.rs"
max-code-lines = 400
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.checks.mise_tasks);
        let rules = config.file_size.unwrap().rules;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].max_code_lines, 400);
    }
}
