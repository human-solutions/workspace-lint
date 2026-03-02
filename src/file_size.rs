use crate::Issue;
use crate::config::FileSizeConfig;
use globset::{Glob, GlobSetBuilder};
use std::collections::HashMap;
use std::process::Command;
use tokei::{Config as TokeiConfig, Languages};

pub fn check(config: &FileSizeConfig) -> Vec<Issue> {
    // Build glob matchers for each rule
    let mut builder = GlobSetBuilder::new();
    for rule in &config.rules {
        builder.add(Glob::new(&rule.glob).unwrap_or_else(|e| {
            eprintln!("invalid glob pattern '{}': {e}", rule.glob);
            std::process::exit(1);
        }));
    }
    let globset = builder.build().unwrap();

    // Use tokei to count all files (respects .gitignore)
    let mut languages = Languages::new();
    languages.get_statistics(&["."], &[], &TokeiConfig::default());

    // Aggregate code lines per file (main + embedded languages)
    let mut file_lines: HashMap<String, usize> = HashMap::new();
    for language in languages.values() {
        for report in &language.reports {
            let path = report.name.strip_prefix("./").unwrap_or(&report.name);
            let key = path.display().to_string();
            *file_lines.entry(key).or_default() += report.stats.code;
        }
        for child_reports in language.children.values() {
            for report in child_reports {
                let path = report.name.strip_prefix("./").unwrap_or(&report.name);
                let key = path.display().to_string();
                *file_lines.entry(key).or_default() += report.stats.code;
            }
        }
    }

    // Check each file against matching rules
    let mut violations: Vec<Vec<(String, usize)>> = vec![Vec::new(); config.rules.len()];

    for (path_str, code_lines) in &file_lines {
        let path = std::path::Path::new(path_str);
        let matches = globset.matches(path);
        for &rule_idx in &matches {
            if *code_lines > config.rules[rule_idx].max_code_lines {
                violations[rule_idx].push((path_str.clone(), *code_lines));
            }
        }
    }

    // Build issues per rule
    let mut issues = Vec::new();
    for (rule_idx, mut viols) in violations.into_iter().enumerate() {
        if viols.is_empty() {
            continue;
        }
        viols.sort_by(|a, b| b.1.cmp(&a.1));

        let rule = &config.rules[rule_idx];
        let mut details: Vec<String> = viols
            .iter()
            .map(|(path, count)| format!("{path}: {count} code lines"))
            .collect();

        details.push(String::new());
        details.push("Suggestions:".to_string());
        details.push("- Move #[cfg(test)] modules to separate test files".to_string());
        details.push(
            "- Extract related structs, enums, or trait impls into their own modules".to_string(),
        );

        issues.push(Issue {
            title: format!(
                "Reduce files matching '{}' to ≤ {} code lines",
                rule.glob, rule.max_code_lines
            ),
            details,
        });
    }

    // Check for deleted files still tracked by git
    issues.extend(check_deleted_files());

    issues
}

fn check_deleted_files() -> Vec<Issue> {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run git ls-files: {e}");
            std::process::exit(1);
        });

    let files = String::from_utf8_lossy(&output.stdout);
    let deleted: Vec<String> = files
        .lines()
        .filter(|path| !std::path::Path::new(path).exists())
        .map(|s| s.to_string())
        .collect();

    if deleted.is_empty() {
        Vec::new()
    } else {
        vec![Issue {
            title: "Stage deleted files with `git rm`".to_string(),
            details: deleted,
        }]
    }
}
