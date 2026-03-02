use crate::Issue;
use fs_err as fs;

pub fn check() -> Vec<Issue> {
    let mise_version = read_mise_version();
    let lock_version = read_lock_version();

    if mise_version != lock_version {
        vec![Issue {
            title: "Fix wasm-bindgen version mismatch".to_string(),
            details: vec![format!(
                "Cargo.lock has {lock_version}, .mise.toml has {mise_version}"
            )],
        }]
    } else {
        vec![]
    }
}

fn read_mise_version() -> String {
    let content = fs::read_to_string(".mise.toml").unwrap_or_else(|e| {
        eprintln!("failed to read .mise.toml: {e}");
        std::process::exit(1);
    });

    let doc: toml::Value = content.parse().unwrap_or_else(|e| {
        eprintln!("failed to parse .mise.toml: {e}");
        std::process::exit(1);
    });

    doc.get("tools")
        .and_then(|t| t.get("cargo:wasm-bindgen-cli"))
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            eprintln!("missing [tools].\"cargo:wasm-bindgen-cli\" in .mise.toml");
            std::process::exit(1);
        })
        .to_string()
}

fn read_lock_version() -> String {
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
        .and_then(|packages| {
            packages
                .iter()
                .find(|pkg| pkg.get("name").and_then(|n| n.as_str()) == Some("wasm-bindgen"))
        })
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            eprintln!("wasm-bindgen not found in Cargo.lock");
            std::process::exit(1);
        })
        .to_string()
}
