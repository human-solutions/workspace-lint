mod claude_md_freshness;
mod file_size;
mod mise_tasks;
mod wasm_bindgen_version;
mod workspace_deps;

pub(crate) struct Issue {
    pub title: String,
    pub details: Vec<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("done") {
        claude_md_freshness::mark_done();
        return;
    }

    mise_tasks::check(); // auto-fixes, exits on hard error only

    let mut issues = Vec::new();
    issues.extend(wasm_bindgen_version::check());
    issues.extend(workspace_deps::check());
    issues.extend(claude_md_freshness::check());
    issues.extend(file_size::check());

    if issues.is_empty() {
        eprintln!("Workspace lint: all passed");
    } else {
        eprintln!(
            "Workspace lint: {} issue{}",
            issues.len(),
            if issues.len() == 1 { "" } else { "s" }
        );
        eprintln!();
        for issue in &issues {
            eprintln!("- [ ] {}", issue.title);
            for detail in &issue.details {
                eprintln!("      {detail}");
            }
            eprintln!();
        }
        if issues.len() > 1 {
            eprintln!("Tip: fix each item in a subagent");
        }
        std::process::exit(1);
    }
}
