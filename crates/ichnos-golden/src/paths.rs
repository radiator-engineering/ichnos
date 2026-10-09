use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The workspace root: the nearest ancestor of this crate whose `Cargo.toml`
/// declares `[workspace]`.
///
/// # Panics
///
/// Panics if no such ancestor exists, which means the crate was moved out of
/// the workspace.
pub fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .ancestors()
            .find(|dir| {
                std::fs::read_to_string(dir.join("Cargo.toml"))
                    .is_ok_and(|text| text.lines().any(|line| line.trim() == "[workspace]"))
            })
            .unwrap_or_else(|| panic!("no workspace Cargo.toml above {}", manifest_dir.display()))
            .to_path_buf()
    })
}

/// `fixtures/logs`, the vendored copy of pm4py's test inputs.
pub fn fixtures_dir() -> PathBuf {
    workspace_root().join("fixtures").join("logs")
}

/// `fixtures/golden`, the generated pm4py outputs.
pub fn golden_dir() -> PathBuf {
    workspace_root().join("fixtures").join("golden")
}

/// The path of a fixture, relative to `fixtures/logs`.
///
/// # Panics
///
/// Panics if the file does not exist, naming the path it looked for.
pub fn fixture_path(rel: impl AsRef<Path>) -> PathBuf {
    let path = fixtures_dir().join(rel.as_ref());
    assert!(
        path.is_file(),
        "fixture not found: {} (fixtures are vendored under fixtures/logs)",
        path.display()
    );
    path
}

/// The path of the golden file for `area`/`case`. It need not exist.
pub fn golden_path(area: &str, case: &str) -> PathBuf {
    golden_dir().join(area).join(format!("{case}.json"))
}
