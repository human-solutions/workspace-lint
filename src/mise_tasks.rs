use fs_err as fs;
use std::process::Command;

const CLAUDE_MD: &str = "CLAUDE.md";
const START_MARKER: &str = "<!-- MISE_TASKS_START -->";
const END_MARKER: &str = "<!-- MISE_TASKS_END -->";

pub fn check() {
    let output = Command::new("mise")
        .args(["tasks"])
        .output()
        .expect("failed to run `mise tasks`");

    if !output.status.success() {
        eprintln!(
            "mise tasks failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    let tasks_output = String::from_utf8_lossy(&output.stdout);
    let table = format_tasks_table(&tasks_output);

    let content = fs::read_to_string(CLAUDE_MD).unwrap_or_else(|e| {
        eprintln!("failed to read {CLAUDE_MD}: {e}");
        std::process::exit(1);
    });

    let Some(start) = content.find(START_MARKER) else {
        eprintln!("{CLAUDE_MD}: missing {START_MARKER}");
        std::process::exit(1);
    };
    let Some(end) = content.find(END_MARKER) else {
        eprintln!("{CLAUDE_MD}: missing {END_MARKER}");
        std::process::exit(1);
    };

    let new_content = format!(
        "{}{START_MARKER}\n{table}{END_MARKER}\n{}",
        &content[..start],
        &content[end + END_MARKER.len()..].trim_start_matches('\n'),
    );

    if new_content == content {
        return;
    }

    fs::write(CLAUDE_MD, &new_content).unwrap_or_else(|e| {
        eprintln!("failed to write {CLAUDE_MD}: {e}");
        std::process::exit(1);
    });

    let status = Command::new("git")
        .args(["add", CLAUDE_MD])
        .status()
        .expect("failed to run `git add`");

    if !status.success() {
        eprintln!("git add {CLAUDE_MD} failed");
        std::process::exit(1);
    }

    eprintln!("updated {CLAUDE_MD} with current mise tasks");
}

fn format_tasks_table(output: &str) -> String {
    let mut table = String::from("| Task | Description |\n|------|-------------|\n");

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // mise tasks output format: "task-name   description text"
        // Split on first run of 2+ spaces
        if let Some(pos) = line.find("  ") {
            let task = line[..pos].trim();
            let desc = line[pos..].trim();
            table.push_str(&format!("| `{task}` | {desc} |\n"));
        }
    }

    table
}
