use crate::Issue;
use fs_err as fs;
use std::fs::FileTimes;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub fn check() -> Vec<Issue> {
    if std::env::var("CI").is_ok() {
        return Vec::new();
    }

    let leaves = find_leaf_claude_mds();

    let mut issues = Vec::new();
    for leaf in &leaves {
        if is_stale(leaf) {
            issues.push(Issue {
                title: format!("Review {}", leaf.display()),
                details: vec![
                    "Source files in subtree are newer".to_string(),
                    "Read the file and update it if the changes affect its accuracy".to_string(),
                    "Run 'workspace-lint done' when done".to_string(),
                ],
            });
        }
    }

    issues
}

pub fn mark_done() {
    let leaves = find_leaf_claude_mds();
    let now = SystemTime::now();
    let times = FileTimes::new().set_modified(now);

    for leaf in &leaves {
        let file = match fs::File::options().write(true).open(leaf) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("failed to open {}: {e}", leaf.display());
                continue;
            }
        };
        if let Err(e) = file.set_times(times) {
            eprintln!("failed to touch {}: {e}", leaf.display());
        }
    }
}

fn find_leaf_claude_mds() -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["ls-files", "**/CLAUDE.md", "CLAUDE.md"])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run git ls-files: {e}");
            std::process::exit(1);
        });

    let stdout = String::from_utf8_lossy(&output.stdout);
    let all: Vec<PathBuf> = stdout.lines().map(PathBuf::from).collect();

    filter_leaves(&all)
}

fn filter_leaves(all: &[PathBuf]) -> Vec<PathBuf> {
    all.iter()
        .filter(|candidate| {
            let dir = candidate.parent().unwrap_or(Path::new(""));
            !all.iter().any(|other| {
                other != *candidate
                    && other.parent().unwrap_or(Path::new("")).starts_with(dir)
                    && other.components().count() > candidate.components().count()
            })
        })
        .cloned()
        .collect()
}

fn is_stale(claude_md: &Path) -> bool {
    let claude_mtime = match mtime(claude_md) {
        Some(t) => t,
        None => return false,
    };

    let dir = claude_md.parent().unwrap_or(Path::new("."));
    let dir_arg = if dir == Path::new("") {
        ".".to_string()
    } else {
        dir.display().to_string()
    };

    let output = Command::new("git")
        .args(["ls-files", &dir_arg])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run git ls-files for {}: {e}", dir.display());
            std::process::exit(1);
        });

    let stdout = String::from_utf8_lossy(&output.stdout);
    let newest = stdout
        .lines()
        .filter(|line| !line.ends_with("CLAUDE.md"))
        .filter_map(|line| mtime(Path::new(line)))
        .max();

    match newest {
        Some(newest) => newest > claude_mtime,
        None => false,
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pb(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn single_root_is_leaf() {
        let all = vec![pb("CLAUDE.md")];
        assert_eq!(filter_leaves(&all), vec![pb("CLAUDE.md")]);
    }

    #[test]
    fn root_with_nested_keeps_only_nested() {
        let all = vec![pb("CLAUDE.md"), pb("crates/tools/workspace-lint/CLAUDE.md")];
        assert_eq!(
            filter_leaves(&all),
            vec![pb("crates/tools/workspace-lint/CLAUDE.md")]
        );
    }

    #[test]
    fn siblings_are_both_leaves() {
        let all = vec![pb("crates/server/CLAUDE.md"), pb("crates/client/CLAUDE.md")];
        let leaves = filter_leaves(&all);
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&pb("crates/server/CLAUDE.md")));
        assert!(leaves.contains(&pb("crates/client/CLAUDE.md")));
    }

    #[test]
    fn deep_nesting_keeps_deepest() {
        let all = vec![
            pb("CLAUDE.md"),
            pb("crates/CLAUDE.md"),
            pb("crates/tools/CLAUDE.md"),
        ];
        assert_eq!(filter_leaves(&all), vec![pb("crates/tools/CLAUDE.md")]);
    }

    #[test]
    fn mixed_depths_filters_correctly() {
        let all = vec![
            pb("CLAUDE.md"),
            pb("crates/server/CLAUDE.md"),
            pb("crates/client/CLAUDE.md"),
            pb("crates/client/logic/CLAUDE.md"),
        ];
        let leaves = filter_leaves(&all);
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&pb("crates/server/CLAUDE.md")));
        assert!(leaves.contains(&pb("crates/client/logic/CLAUDE.md")));
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(filter_leaves(&[]).is_empty());
    }

    #[test]
    fn similar_prefix_not_confused_as_ancestor() {
        let all = vec![
            pb("crates/client/CLAUDE.md"),
            pb("crates/client-foo/CLAUDE.md"),
        ];
        let leaves = filter_leaves(&all);
        assert_eq!(leaves.len(), 2);
    }
}
