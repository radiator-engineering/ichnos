use std::collections::BTreeSet;

use super::*;

fn sample() -> (Dfg, ActivityCounts) {
    let mut dfg = Dfg::new();
    for (a, b, n) in [
        ("a", "b", 10),
        ("b", "c", 8),
        ("c", "d", 9),
        ("b", "d", 2),
        ("a", "e", 3),
        ("e", "d", 3),
        ("d", "b", 1),
        ("c", "c", 4),
        ("e", "f", 1),
        ("f", "d", 1),
    ] {
        dfg.add_edge(a, b, n);
    }
    dfg.add_start("a", 13);
    dfg.add_end("d", 12);
    dfg.add_end("c", 1);
    let counts = [
        ("a", 13),
        ("b", 11),
        ("c", 12),
        ("d", 15),
        ("e", 3),
        ("f", 1),
    ]
    .into_iter()
    .map(|(a, n)| (Label::from(a), n))
    .collect();
    (dfg, counts)
}

/// Formats a filter result as `edges|starts|ends|counts`, the format the
/// pm4py reference values below use.
fn show((dfg, counts): (Dfg, ActivityCounts)) -> String {
    let map = |m: &BTreeMap<Label, u64>| {
        m.iter()
            .map(|(a, n)| format!("{a}:{n}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let edges = dfg
        .graph
        .iter()
        .map(|((a, b), n)| format!("{a}>{b}:{n}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{edges}|{}|{}|{}",
        map(&dfg.start_activities),
        map(&dfg.end_activities),
        map(&counts)
    )
}

#[test]
fn filters_match_pm4py() {
    let (dfg, counts) = sample();
    // Expected values from pm4py 2.7.23.8 `algo/filtering/dfg/dfg_filtering.py`.
    let cases: Vec<(&str, (Dfg, ActivityCounts), &str)> = vec![
        (
            "act0.0",
            dfg.filter_activities_percentage(&counts, 0.0),
            "a>b:10,b>d:2,d>b:1|a:13|d:12|a:13,b:11,d:15",
        ),
        (
            "act0.3",
            dfg.filter_activities_percentage(&counts, 0.3),
            "a>b:10,b>c:8,b>d:2,c>c:4,c>d:9,d>b:1|a:13|c:1,d:12|a:13,b:11,c:12,d:15",
        ),
        (
            "act0.6",
            dfg.filter_activities_percentage(&counts, 0.6),
            "a>b:10,b>c:8,b>d:2,c>c:4,c>d:9,d>b:1|a:13|c:1,d:12|a:13,b:11,c:12,d:15",
        ),
        (
            "act1.0",
            dfg.filter_activities_percentage(&counts, 1.0),
            "a>b:10,a>e:3,b>c:8,b>d:2,c>c:4,c>d:9,d>b:1,e>d:3,e>f:1,f>d:1|a:13|c:1,d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "path0.0False",
            dfg.filter_paths_percentage(&counts, 0.0, false),
            "a>b:10,b>c:8,c>d:9|a:13|d:12|a:13,b:11,c:12,d:15",
        ),
        (
            "path0.0True",
            dfg.filter_paths_percentage(&counts, 0.0, true),
            "a>b:10,a>e:3,b>c:8,c>d:9,e>f:1,f>d:1|a:13|d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "path0.3False",
            dfg.filter_paths_percentage(&counts, 0.3, false),
            "a>b:10,b>c:8,c>d:9|a:13|d:12|a:13,b:11,c:12,d:15",
        ),
        (
            "path0.3True",
            dfg.filter_paths_percentage(&counts, 0.3, true),
            "a>b:10,a>e:3,b>c:8,c>d:9,e>f:1,f>d:1|a:13|d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "path0.6False",
            dfg.filter_paths_percentage(&counts, 0.6, false),
            "a>b:10,a>e:3,b>c:8,b>d:2,c>c:4,c>d:9,e>d:3|a:13|d:12|a:13,b:11,c:12,d:15,e:3",
        ),
        (
            "path0.6True",
            dfg.filter_paths_percentage(&counts, 0.6, true),
            "a>b:10,a>e:3,b>c:8,b>d:2,c>c:4,c>d:9,e>d:3,e>f:1,f>d:1|a:13|d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "conn0.5False",
            dfg.filter_keep_connected(&counts, 0.5, false),
            "a>b:10,a>e:3,b>c:8,c>d:9,e>d:3,e>f:1,f>d:1|a:13|c:1,d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "conn0.5True",
            dfg.filter_keep_connected(&counts, 0.5, true),
            "a>b:10,a>e:3,b>c:8,c>d:9,e>d:3,e>f:1,f>d:1|a:13|c:1,d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "conn0.8False",
            dfg.filter_keep_connected(&counts, 0.8, false),
            "a>b:10,b>c:8,c>d:9|a:13|c:1,d:12|a:13,b:11,c:12,d:15",
        ),
        (
            "conn0.8True",
            dfg.filter_keep_connected(&counts, 0.8, true),
            "a>b:10,a>e:3,b>c:8,c>d:9,e>f:1,f>d:1|a:13|c:1,d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "to_c",
            dfg.filter_to_activity(&counts, "c").unwrap(),
            "a>b:10,a>e:3,b>c:8,b>d:2,d>b:1,e>d:3,e>f:1,f>d:1|a:13|c:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
        (
            "from_b",
            dfg.filter_from_activity(&counts, "b").unwrap(),
            "b>c:8,b>d:2,c>c:4,c>d:9|b:11|c:1,d:12|b:11,c:12,d:15",
        ),
        (
            "contain_e",
            dfg.filter_contain_activity(&counts, "e").unwrap(),
            "a>b:10,a>e:3,b>c:8,b>d:2,c>c:4,c>d:9,d>b:1,e>d:3,e>f:1,f>d:1|a:13|c:1,d:12|a:13,b:11,c:12,d:15,e:3,f:1",
        ),
    ];
    for (name, got, expected) in cases {
        assert_eq!(show(got), expected, "{name}");
    }
    assert_eq!(
        dfg.filter_to_activity(&counts, "zz"),
        Err(DfgError::UnknownActivity("zz".into()))
    );
}

#[test]
fn noise_cleaning_matches_pm4py() {
    let (dfg, _) = sample();
    let cleaned = dfg.clean_noise(0.2, &BTreeSet::new());
    let mut expected = dfg.graph.clone();
    expected.remove(&("d".into(), "b".into()));
    assert_eq!(cleaned.graph, expected);
    let keep = BTreeSet::from([(Label::from("d"), Label::from("b"))]);
    assert_eq!(dfg.clean_noise(0.2, &keep).graph, dfg.graph);
}

#[test]
fn structure_helpers() {
    let (dfg, _) = sample();
    let set = |xs: &[&str]| xs.iter().map(|&x| Label::from(x)).collect::<BTreeSet<_>>();
    assert_eq!(dfg.vertices(), set(&["a", "b", "c", "d", "e", "f"]));
    assert_eq!(dfg.source_vertices(), set(&["a"]));
    assert!(dfg.sink_vertices().is_empty());
    assert_eq!(dfg.infer_start_activities(), set(&["a"]));
    assert!(dfg.infer_end_activities().is_empty());
    assert_eq!(dfg.max_activity_count("e"), Some(3));
    assert_eq!(dfg.max_activity_count("zz"), None);
    assert_eq!(dfg.outgoing("b").len(), 2);
    assert_eq!(dfg.incoming("d").len(), 4);
    let succ = dfg.successors();
    assert_eq!(succ[&Label::from("c")], set(&["b", "c", "d"]));
    assert_eq!(succ[&Label::from("a")], set(&["b", "c", "d", "e", "f"]));
    let pred = dfg.predecessors();
    assert_eq!(pred[&Label::from("a")], set(&[]));
    assert_eq!(pred[&Label::from("f")], set(&["a", "e"]));
    let freq = dfg.vertex_frequencies();
    assert_eq!(freq[&Label::from("a")], 26);
    assert_eq!(freq[&Label::from("f")], 1);
}
