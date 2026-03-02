use crate::Issue;
use crate::config::FreshnessConfig;
use globset::Glob;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn check(config: &FreshnessConfig) -> Vec<Issue> {
    if std::env::var("CI").is_ok() {
        return Vec::new();
    }

    let mut issues = Vec::new();

    for rule in &config.rules {
        let tracked_files = find_files_matching(&rule.glob);

        for file in &tracked_files {
            let file_mtime = match mtime(file) {
                Some(t) => t,
                None => continue,
            };

            let parent = file.parent().unwrap_or(Path::new("."));
            let parent_dir = if parent == Path::new("") {
                Path::new(".")
            } else {
                parent
            };

            let dep_files = find_deps_in_dir(parent_dir, &rule.depends_on);
            let newest_dep = dep_files.iter().filter_map(|p| mtime(p)).max();

            if let Some(newest) = newest_dep
                && newest > file_mtime
            {
                issues.push(Issue {
                    title: format!("Review {}", file.display()),
                    details: vec![
                        format!(
                            "Source files matching {} in subtree are newer",
                            rule.depends_on
                        ),
                        "Run 'workspace-lint done' when done".to_string(),
                    ],
                });
            }
        }
    }

    issues
}

pub fn mark_done(config: &FreshnessConfig) {
    let now = SystemTime::now();
    let times = std::fs::FileTimes::new().set_modified(now);

    for rule in &config.rules {
        let files = find_files_matching(&rule.glob);
        for file in &files {
            let f = match fs_err::File::options().write(true).open(file) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("failed to open {}: {e}", file.display());
                    continue;
                }
            };
            if let Err(e) = f.set_times(times) {
                eprintln!("failed to touch {}: {e}", file.display());
            }
        }
    }
}

/// Find all tracked files matching a glob pattern, walking with .gitignore respect.
fn find_files_matching(pattern: &str) -> Vec<PathBuf> {
    let glob = Glob::new(pattern).unwrap_or_else(|e| {
        eprintln!("invalid glob pattern '{pattern}': {e}");
        std::process::exit(1);
    });
    let matcher = glob.compile_matcher();

    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(".").build().flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path().strip_prefix("./").unwrap_or(entry.path());
        if matcher.is_match(path) {
            results.push(path.to_path_buf());
        }
    }
    results
}

/// Find dependency files in a directory matching a glob pattern.
fn find_deps_in_dir(dir: &Path, pattern: &str) -> Vec<PathBuf> {
    let glob = Glob::new(pattern).unwrap_or_else(|e| {
        eprintln!("invalid depends-on pattern '{pattern}': {e}");
        std::process::exit(1);
    });
    let matcher = glob.compile_matcher();

    let mut results = Vec::new();
    for entry in ignore::WalkBuilder::new(dir).build().flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let rel = match entry.path().strip_prefix(dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if matcher.is_match(rel) {
            results.push(entry.into_path());
        }
    }
    results
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs_err::metadata(path).ok()?.modified().ok()
}
