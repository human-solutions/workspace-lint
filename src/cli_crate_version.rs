use crate::Issue;
use crate::config::CliCrateVersionConfig;
use fs_err as fs;
use regex::Regex;
use std::process::Command;

pub fn check(config: &CliCrateVersionConfig) -> Vec<Issue> {
    let lock_packages = read_lock_packages();
    let mut issues = Vec::new();

    for rule in &config.rules {
        let (program, args) = rule.command.split_first().unwrap_or_else(|| {
            eprintln!("cli-crate-version: command must not be empty");
            std::process::exit(1);
        });

        let output = Command::new(program)
            .args(args)
            .output()
            .unwrap_or_else(|e| {
                eprintln!(
                    "cli-crate-version: failed to run `{}`: {e}",
                    rule.command.join(" ")
                );
                std::process::exit(1);
            });

        if !output.status.success() {
            eprintln!(
                "cli-crate-version: `{}` failed: {}",
                rule.command.join(" "),
                String::from_utf8_lossy(&output.stderr)
            );
            std::process::exit(1);
        }

        let raw = strip_ansi_escapes::strip(&output.stdout);
        let stdout = String::from_utf8_lossy(&raw);

        let re = Regex::new(&rule.pattern).unwrap_or_else(|e| {
            eprintln!(
                "cli-crate-version: invalid regex pattern `{}`: {e}",
                rule.pattern
            );
            std::process::exit(1);
        });

        let cli_version = re
            .captures(&stdout)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .unwrap_or_else(|| {
                eprintln!(
                    "cli-crate-version: pattern `{}` did not match output of `{}`",
                    rule.pattern,
                    rule.command.join(" ")
                );
                std::process::exit(1);
            });

        let lock_version = lock_packages
            .iter()
            .find(|(name, _)| name == &rule.crate_name)
            .map(|(_, version)| version.as_str())
            .unwrap_or_else(|| {
                eprintln!(
                    "cli-crate-version: crate `{}` not found in Cargo.lock",
                    rule.crate_name
                );
                std::process::exit(1);
            });

        if cli_version != lock_version {
            issues.push(Issue {
                title: format!("Fix {} version mismatch", rule.crate_name),
                details: vec![format!(
                    "CLI reports {cli_version}, Cargo.lock has {lock_version}"
                )],
            });
        }
    }

    issues
}

fn read_lock_packages() -> Vec<(String, String)> {
    let content = fs::read_to_string("Cargo.lock").unwrap_or_else(|e| {
        eprintln!("failed to read Cargo.lock: {e}");
        std::process::exit(1);
    });

    let doc: toml::Value = content.parse().unwrap_or_else(|e| {
        eprintln!("failed to parse Cargo.lock: {e}");
        std::process::exit(1);
    });

    doc.get("package")
        .and_then(|p| p.as_array())
        .map(|packages| {
            packages
                .iter()
                .filter_map(|pkg| {
                    let name = pkg.get("name")?.as_str()?;
                    let version = pkg.get("version")?.as_str()?;
                    Some((name.to_string(), version.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}
