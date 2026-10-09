//! OTG and ET-OT discovery against the pm4py goldens
//! `discovery/{otg,etot}-*`.

use std::path::Path;

use ichnos_discovery::{discover_etot, discover_otg};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_ocel::Ocel;
use serde_json::{Value, json};

const LOGS: [&str; 2] = ["example-log", "ocel20-example"];

fn read(path: &Path) -> Ocel {
    if path.to_string_lossy().contains("ocel20") {
        ichnos_io::read_ocel2_json(path).unwrap()
    } else {
        ichnos_io::read_ocel_json(path).unwrap()
    }
}

#[test]
fn oracle_discover_otg() {
    for log in LOGS {
        let g = golden("discovery", &format!("otg-{log}"));
        let otg = discover_otg(&read(&g.fixture("log"))).unwrap();
        let actual = json!({
            "object_types": otg.object_types,
            "edges": otg.edges.iter().map(|(e, n)| json!({
                "source": e.source, "relation": e.relation.name(), "target": e.target, "count": n,
            })).collect::<Vec<_>>(),
        });
        let expected: Value = g.expected_as();
        assert_json_eq(&actual, &expected, &JsonCompare::default().unordered());
    }
}

#[test]
fn oracle_discover_etot() {
    for log in LOGS {
        let g = golden("discovery", &format!("etot-{log}"));
        let etot = discover_etot(&read(&g.fixture("log"))).unwrap();
        let actual = json!({
            "activities": etot.activities,
            "object_types": etot.object_types,
            "edges": etot.edges.iter().map(|((a, ot), n)| json!({
                "activity": a, "object_type": ot, "count": n,
            })).collect::<Vec<_>>(),
        });
        let expected: Value = g.expected_as();
        assert_json_eq(&actual, &expected, &JsonCompare::default().unordered());
    }
}
