use crate::Issue;
use std::process::Command;

const MAX_CODE_LINES: usize = 500;

pub fn check() -> Vec<Issue> {
    let output = Command::new("git")
        .args(["ls-files", "*.rs"])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run git ls-files: {e}");
            std::process::exit(1);
        });

    let files = String::from_utf8_lossy(&output.stdout);
    let mut violations: Vec<(String, usize)> = Vec::new();
    let mut deleted: Vec<String> = Vec::new();

    for path in files.lines() {
        if !std::path::Path::new(path).exists() {
            deleted.push(path.to_string());
            continue;
        }
        let content = fs_err::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("failed to read {path}: {e}");
            std::process::exit(1);
        });
        let count = count_code_lines(&content);
        if count > MAX_CODE_LINES {
            violations.push((path.to_string(), count));
        }
    }

    let mut issues: Vec<Issue> = Vec::new();

    if !violations.is_empty() {
        violations.sort_by(|a, b| b.1.cmp(&a.1));

        let mut details: Vec<String> = violations
            .iter()
            .map(|(path, count)| format!("{path}: {count} code lines"))
            .collect();

        details.push(String::new());
        details.push("Suggestions:".to_string());
        details.push("- Move #[cfg(test)] modules to separate test files".to_string());
        details.push(
            "- Extract related structs, enums, or trait impls into their own modules".to_string(),
        );
        details.push(
            "- Use extension traits to attach feature-specific methods to structs and enums"
                .to_string(),
        );

        issues.push(Issue {
            title: format!("Reduce .rs file sizes to ≤ {MAX_CODE_LINES} code lines"),
            details,
        });
    }

    if !deleted.is_empty() {
        issues.push(Issue {
            title: "Stage deleted files with `git rm`".to_string(),
            details: deleted,
        });
    }

    issues
}

fn count_code_lines(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}
