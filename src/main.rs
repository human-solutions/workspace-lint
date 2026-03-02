mod centralized_deps;
mod cli_crate_version;
mod config;
mod crate_size;
mod expand;
mod file_size;
mod freshness;

pub(crate) struct Issue {
    pub title: String,
    pub details: Vec<String>,
}

fn main() {
    let config = config::load();
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("done") {
        if let Some(ref fc) = config.freshness {
            freshness::mark_done(fc);
        }
        return;
    }

    if let Some(ref ec) = config.expand {
        expand::run(ec);
    }

    let mut issues = Vec::new();

    if let Some(ref cv) = config.cli_crate_version {
        issues.extend(cli_crate_version::check(cv));
    }
    if config.checks.centralized_deps {
        issues.extend(centralized_deps::check());
    }
    if let Some(ref fc) = config.freshness {
        issues.extend(freshness::check(fc));
    }
    if let Some(ref fc) = config.file_size {
        issues.extend(file_size::check(fc));
    }
    if let Some(ref fc) = config.crate_size {
        issues.extend(crate_size::check(fc));
    }

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
