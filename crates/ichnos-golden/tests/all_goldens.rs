//! Every golden file in `fixtures/golden` loads, and its meta block names it and real fixtures.

use ichnos_golden::{areas, cases, golden, workspace_root};

#[test]
fn every_golden_file_is_well_formed() {
    let areas = areas();
    assert!(
        areas.iter().any(|a| a == "log"),
        "no log area; found {areas:?}"
    );
    let mut seen = 0;
    for area in &areas {
        for case in cases(area) {
            let g = golden(area, &case);
            let meta = g.meta();
            assert_eq!(meta.area, *area, "{}", g.path.display());
            assert_eq!(meta.case, case, "{}", g.path.display());
            assert!(
                !meta.functions.is_empty(),
                "{}: no functions",
                g.path.display()
            );
            for (role, rel) in &meta.fixtures {
                assert!(
                    workspace_root().join(rel).is_file(),
                    "{}: fixture `{role}` ({rel}) does not exist",
                    g.path.display()
                );
                assert!(
                    meta.loaders.contains_key(role),
                    "{}: no loader for `{role}`",
                    g.path.display()
                );
            }
            seen += 1;
        }
    }
    assert!(seen > 0);
}

#[test]
fn missing_golden_is_a_clear_error() {
    let err = ichnos_golden::try_golden("log", "no-such-case").unwrap_err();
    let text = err.to_string();
    assert!(text.contains("no-such-case.json"), "{text}");
}

#[test]
#[should_panic(expected = "fixture not found")]
fn missing_fixture_panics() {
    ichnos_golden::fixture_path("no-such-file.xes");
}

#[test]
fn fixture_path_finds_vendored_logs() {
    assert!(ichnos_golden::fixture_path("running-example.xes").is_file());
    assert!(ichnos_golden::fixture_path("synthetic_logs/a12/a12f0n00.xes.gz").is_file());
}
