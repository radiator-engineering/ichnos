use ichnos_core::{AttributeValue, Attributes, EventLog};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

fn digest(value: &Value) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}

fn types<'a>(objects: impl IntoIterator<Item = &'a Attributes>) -> Value {
    let mut result: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for attrs in objects {
        for (key, value) in attrs {
            result
                .entry(key.to_string())
                .or_default()
                .insert(value.type_name().to_owned());
        }
    }
    serde_json::to_value(result).unwrap()
}

fn attribute(value: &AttributeValue) -> Value {
    match value {
        AttributeValue::String(s) | AttributeValue::Id(s) => json!(s.as_ref()),
        AttributeValue::Int(n) => json!(n),
        AttributeValue::Float(n) => json!(n),
        AttributeValue::Bool(b) => json!(b),
        AttributeValue::Date(d) => json!(
            d.to_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
        ),
        AttributeValue::List(items) => {
            json!({"list": items.iter().map(|(k,v)| json!([k.as_ref(), attribute(v)])).collect::<Vec<_>>()})
        }
        AttributeValue::Container(children) => json!({"container": attributes(children)}),
        AttributeValue::Meta(meta) => {
            json!({"value": attribute(&meta.value), "meta": attributes(&meta.meta)})
        }
    }
}

fn attributes(attrs: &Attributes) -> Value {
    Value::Object(
        attrs
            .iter()
            .map(|(k, v)| (k.to_string(), attribute(v)))
            .collect(),
    )
}

pub fn assert_golden(log: &EventLog, expected: &Value, fixture: &str) {
    assert_eq!(json!(log.len()), expected["n_cases"], "{fixture}: cases");
    assert_eq!(
        json!(log.num_events()),
        expected["n_events"],
        "{fixture}: events"
    );
    let sequences: Vec<Vec<Option<String>>> = log
        .traces
        .iter()
        .map(|t| {
            t.events
                .iter()
                .map(|e| e.get("concept:name").map(ToString::to_string))
                .collect()
        })
        .collect();
    assert_eq!(
        digest(&json!(sequences)),
        expected["sequence_sha256"],
        "{fixture}: ordered sequences"
    );
    let mut variants = BTreeMap::<String, u64>::new();
    let mut profiles = BTreeMap::<String, Value>::new();
    for (trace, sequence) in log.traces.iter().zip(&sequences) {
        *variants.entry(digest(&json!(sequence))).or_default() += 1;
        let mut profile = json!({"trace": types([&trace.attributes]), "events": types(trace.events.iter().map(|e| &e.attributes))});
        let key = digest(&profile);
        profile["count"] = json!(0);
        let entry = profiles.entry(key).or_insert(profile);
        entry["count"] = json!(entry["count"].as_u64().unwrap() + 1);
    }
    assert_eq!(json!(variants), expected["variants"], "{fixture}: variants");
    assert_eq!(
        json!(profiles),
        expected["type_profiles"],
        "{fixture}: per-trace types"
    );
    assert_eq!(
        types(log.traces.iter().map(|t| &t.attributes)),
        expected["trace_types"],
        "{fixture}: trace types"
    );
    assert_eq!(
        types(
            log.traces
                .iter()
                .flat_map(|t| t.events.iter().map(|e| &e.attributes))
        ),
        expected["event_types"],
        "{fixture}: event types"
    );
    let samples: Vec<Value> = log.traces.iter().take(3).map(|t| json!({"attributes": attributes(&t.attributes), "events": t.events.iter().map(|e| attributes(&e.attributes)).collect::<Vec<_>>()})).collect();
    assert_eq!(
        json!(samples),
        expected["samples"],
        "{fixture}: reference attribute values"
    );
}
