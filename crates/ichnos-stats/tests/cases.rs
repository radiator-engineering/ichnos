mod common;
use common::load;
use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_stats::{
    attributes::KdeOptions,
    cases::*,
    time::{BusinessHours, TimeOptions},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn descriptions(values: indexmap::IndexMap<String, CaseDescription>) -> Value {
    json!(values.into_iter().map(|(id,d)| (id,json!({"startTime":d.start_time,"endTime":d.end_time,"caseDuration":d.case_duration}))).collect::<BTreeMap<_,_>>())
}
#[test]
fn oracle_case_statistics() {
    for name in [
        "running-example",
        "receipt",
        "roadtraffic100traces",
        "interval_event_log",
    ] {
        let log = load(name);
        let keys = EventKeys::default();
        let options = CaseOptions::default();
        let e: Value = golden("stats", &format!("cases-{}", name.replace('_', "-"))).expected_as();
        let check = |field: &str, v: Value| assert_json_eq(&v, &e[field], &JsonCompare::default());
        check(
            "descriptions",
            descriptions(get_cases_description(&log, &keys, &options).unwrap()),
        );
        for (i, sort) in [
            CaseSort::Id,
            CaseSort::Start,
            CaseSort::End,
            CaseSort::Duration,
        ]
        .into_iter()
        .enumerate()
        {
            let opts = CaseOptions {
                sort: Some(sort),
                descending: true,
                max_cases: Some(3),
                ..Default::default()
            };
            let actual=get_cases_description(&log,&keys,&opts).unwrap().into_iter().map(|(id,d)| json!([id,{"startTime":d.start_time,"endTime":d.end_time,"caseDuration":d.case_duration}])).collect::<Vec<_>>();
            assert_json_eq(
                &json!(actual),
                &e["description_options"][i],
                &JsonCompare::default(),
            );
        }
        check(
            "durations",
            json!(get_all_case_durations(&log, &keys, &options).unwrap()),
        );
        let business = TimeOptions {
            business_hours: Some(BusinessHours::default()),
            ..Default::default()
        };
        check(
            "business_durations",
            json!(
                get_all_case_durations(
                    &log,
                    &keys,
                    &CaseOptions {
                        time: business.clone(),
                        ..Default::default()
                    }
                )
                .unwrap()
            ),
        );
        let individual = get_cases_description(&log, &keys, &options)
            .unwrap()
            .into_keys()
            .map(|id| {
                let duration = get_case_duration(&log, &keys, &id, &options).unwrap();
                (id, duration)
            })
            .collect::<BTreeMap<_, _>>();
        check("individual", json!(individual));
        check(
            "quartile",
            json!(get_first_quartile_case_duration(&log, &keys, &options).unwrap()),
        );
        check(
            "median",
            json!(get_median_case_duration(&log, &keys, &options).unwrap()),
        );
        check(
            "arrival",
            json!(get_case_arrival_average(&log, &keys, &TimeOptions::default()).unwrap()),
        );
        check(
            "dispersion",
            json!(get_case_dispersion_average(&log, &keys, &TimeOptions::default()).unwrap()),
        );
        check(
            "business_arrival",
            json!(get_case_arrival_average(&log, &keys, &business).unwrap()),
        );
        check(
            "business_dispersion",
            json!(get_case_dispersion_average(&log, &keys, &business).unwrap()),
        );
        let variants=get_variant_statistics(&log,&keys,VariantStatisticsOptions { include_durations:true,..Default::default() }).unwrap().into_iter().map(|v| json!({"variant":v.variant,"count":v.count,"caseDuration":v.case_duration.unwrap()})).collect::<Vec<_>>();
        check("variants", json!(variants));
        let variants = get_variant_statistics(
            &log,
            &keys,
            VariantStatisticsOptions {
                maximum: Some(2),
                ..Default::default()
            },
        )
        .unwrap()
        .into_iter()
        .map(|v| json!({"variant":v.variant,"count":v.count}))
        .collect::<Vec<_>>();
        check("variants_limited", json!(variants));
        let rows = get_variants_df_with_case_duration(&log, &keys)
            .unwrap()
            .into_iter()
            .map(|v| json!({"case_id":v.case_id,"variant":v.variant,"duration":v.case_duration}))
            .collect::<Vec<_>>();
        check("variant_rows", json!(rows));
        let (rows, list) = get_variants_df_and_list(&log, &keys).unwrap();
        assert_eq!(rows.len(), log.len());
        check(
            "variant_list",
            json!(
                list.into_iter()
                    .map(|v| (v.variant, v.count))
                    .collect::<Vec<_>>()
            ),
        );
        let indexed = index_log_caseid(&log, "concept:name").unwrap();
        check(
            "indexed_lengths",
            json!(
                indexed
                    .iter()
                    .map(|(k, t)| (k.clone(), t.events.len()))
                    .collect::<BTreeMap<_, _>>()
            ),
        );
        let events = indexed
            .into_keys()
            .map(|id| {
                let activities = get_events(&log, &id, "concept:name")
                    .unwrap()
                    .iter()
                    .map(|e| e.get(&keys.activity).unwrap().to_string())
                    .collect::<Vec<_>>();
                (id, activities)
            })
            .collect::<BTreeMap<_, _>>();
        check("events", json!(events));
        check(
            "self_distances",
            json!(get_minimum_self_distances(&log, &keys).unwrap()),
        );
        check(
            "witnesses",
            json!(get_minimum_self_distance_witnesses(&log, &keys).unwrap()),
        );
        check(
            "rework",
            json!(get_rework_cases_per_activity(&log, &keys).unwrap()),
        );
        for activity in e["positions"].as_object().unwrap().keys() {
            assert_eq!(
                json!(get_activity_position_summary(&log, &keys, activity).unwrap()),
                e["positions"][activity]
            );
        }
        let kde = KdeOptions {
            graph_points: 20,
            ..Default::default()
        };
        let density = get_kde_caseduration(&log, &keys, &options, kde).unwrap();
        check("kde", json!([density.x, density.y]));
        for (i, values) in [vec![], vec![5.0], vec![0.0, 1.0, 4.0, 8.0]]
            .iter()
            .enumerate()
        {
            let d = get_kde_case_duration_values(values, kde).unwrap();
            assert_json_eq(
                &json!([d.x, d.y]),
                &e["kde_values"][i],
                &JsonCompare::default(),
            );
        }
    }
}
#[test]
fn empty_case_and_missing_timestamp() {
    let keys = EventKeys::default();
    let options = CaseOptions::default();
    let log = EventLog::default();
    assert!(
        get_all_case_durations(&log, &keys, &options)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        get_case_arrival_average(&log, &keys, &TimeOptions::default()).unwrap(),
        0.0
    );
    assert_eq!(
        get_first_quartile_case_duration(&log, &keys, &options).unwrap(),
        0.0
    );
    assert!(get_case_duration(&log, &keys, "unknown", &options).is_err());
    let mut log = EventLog::from_trace_strings(["A,B,A,C,A", "D,D"], ",", &keys);
    assert_eq!(get_minimum_self_distances(&log, &keys).unwrap()["A"], 1);
    assert_eq!(
        get_minimum_self_distance_witnesses(&log, &keys).unwrap()["A"].len(),
        2
    );
    log.traces[0].events[0].attributes.remove(&keys.timestamp);
    assert!(matches!(
        get_cases_description(&log, &keys, &options),
        Err(ichnos_stats::Error::Core(
            ichnos_core::Error::MissingAttribute { .. }
        ))
    ));
    assert!(get_kde_case_duration_values(&[1.0, 1.0], KdeOptions::default()).is_err());
}
