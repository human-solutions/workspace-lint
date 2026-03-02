use fs_err as fs;
use std::path::{Path, PathBuf};

pub fn extract_member_patterns(root: &toml::Value) -> Vec<String> {
    root.get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

pub fn expand_member_patterns(patterns: &[String]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for pattern in patterns {
        if pattern.contains('*') {
            // Simple glob: "crates/server/*" → list entries in "crates/server/"
            let parent = pattern.trim_end_matches("/*").trim_end_matches("\\*");
            let parent_path = Path::new(parent);
            if let Ok(entries) = fs::read_dir(parent_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("Cargo.toml").exists() {
                        dirs.push(path);
                    }
                }
            }
        } else {
            let path = Path::new(pattern);
            if path.is_dir() {
                dirs.push(path.to_path_buf());
            }
        }
    }

    dirs.sort();
    dirs
}
