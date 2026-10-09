use ichnos_golden::{JsonCompare, assert_json_eq, cases, golden};
use ichnos_stats::cube::{Aggregation, Axis, FeatureTable, get_process_cube};
use serde_json::json;
#[test]
fn cubes_match_numeric_prefix_and_missing_value_goldens() {
    for id in cases("simulation")
        .into_iter()
        .filter(|s| s.starts_with("cube-"))
    {
        let g = golden("simulation", &id);
        let e = &g.expected;
        let data = &e["data"];
        let table = FeatureTable {
            case_ids: serde_json::from_value(data["case:concept:name"].clone()).unwrap(),
            columns: ["nx", "ny", "px_b", "px_a", "py_u", "py_v", "value"]
                .into_iter()
                .filter(|k| data.get(*k).is_some())
                .map(|k| {
                    (
                        k.to_string(),
                        serde_json::from_value(data[k].clone()).unwrap(),
                    )
                })
                .collect(),
        };
        let x = Axis {
            divisions: 2,
            ..Axis::new(e["x"].as_str().unwrap())
        };
        let y = Axis {
            divisions: 2,
            ..Axis::new(e["y"].as_str().unwrap())
        };
        let agg = match e["aggregation"].as_str().unwrap() {
            "mean" => Aggregation::Mean,
            "sum" => Aggregation::Sum,
            "min" => Aggregation::Min,
            "max" => Aggregation::Max,
            _ => unreachable!(),
        };
        let cube = get_process_cube(&table, &x, &y, "value", agg).unwrap();
        assert_eq!(json!([cube.y.len(), cube.x.len()]), e["shape"]);
        assert_json_eq(&json!(cube.values), &e["values"], &JsonCompare::default());
        assert_eq!(json!(cube.cases), e["cases"], "{id}");
    }
}
#[test]
fn manual_bins_constant_columns_and_errors() {
    let t = FeatureTable {
        case_ids: vec!["a".into(), "a".into(), "b".into()],
        columns: vec![
            ("x".into(), vec![Some(1.), Some(1.), Some(1.)]),
            ("y".into(), vec![Some(0.), Some(2.), None]),
            ("v".into(), vec![None, Some(7.), Some(2.)]),
        ],
    };
    let x = Axis::new("x");
    let y = Axis {
        boundaries: Some(vec![2., 0., 1., 1.]),
        ..Axis::new("y")
    };
    let c = get_process_cube(&t, &x, &y, "v", Aggregation::Sum).unwrap();
    assert_eq!(c.values, vec![vec![Some(0.)], vec![Some(7.)]]);
    assert!(get_process_cube(&t, &x, &y, "absent", Aggregation::Mean).is_err());
    let no_axis = Axis::new("missing");
    assert!(
        get_process_cube(&t, &x, &no_axis, "v", Aggregation::Mean)
            .unwrap()
            .values
            .is_empty()
    );
    let mut invalid = t;
    invalid.columns[0].1.pop();
    assert!(get_process_cube(&invalid, &x, &y, "v", Aggregation::Mean).is_err());
}
