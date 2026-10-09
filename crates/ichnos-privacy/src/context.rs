use crate::{PrivacyError, PrivacyOptions, matching, mechanisms};
use chrono::{DateTime, Utc};
use ichnos_core::{AttributeValue, Event, EventLog, Trace};
use rand::{Rng, RngExt};
use std::collections::{BTreeMap, VecDeque};
type Pools = BTreeMap<String, Vec<AttributeValue>>;
fn activity(e: &Event) -> &str {
    e.get("concept:name")
        .and_then(AttributeValue::as_str)
        .expect("query validated activity")
}
fn timestamp(e: &Event) -> Result<i64, PrivacyError> {
    e.get("time:timestamp")
        .and_then(AttributeValue::as_date)
        .map(|t| t.timestamp_micros())
        .ok_or_else(|| PrivacyError::Input("missing timestamp".into()))
}
fn set_timestamp(e: &mut Event, micros: i64) -> Result<(), PrivacyError> {
    let date = DateTime::<Utc>::from_timestamp_micros(micros)
        .ok_or_else(|| PrivacyError::Input("timestamp out of range".into()))?;
    e.insert("time:timestamp", date.fixed_offset());
    Ok(())
}
fn scalar(v: &AttributeValue) -> bool {
    matches!(
        v,
        AttributeValue::String(_) | AttributeValue::Int(_) | AttributeValue::Bool(_)
    ) || matches!(v,AttributeValue::Float(x) if x.is_finite())
}
pub(crate) fn enrich<R: Rng + ?Sized>(
    log: &EventLog,
    query: &[Vec<String>],
    o: &PrivacyOptions,
    rng: &mut R,
) -> Result<EventLog, PrivacyError> {
    if query
        .len()
        .checked_mul(log.traces.len())
        .is_none_or(|n| n > o.max_matching_cells)
    {
        return Err(PrivacyError::Limit("matching cells"));
    }
    let mut source = log.clone();
    let mut labels = Vec::new();
    let mut pools: Pools = BTreeMap::new();
    let mut structures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut times = Vec::new();
    let mut delays = Vec::new();
    let mut pair_delays: BTreeMap<(String, String), Vec<i64>> = BTreeMap::new();
    for trace in &mut source.traces {
        let mut last: Option<(String, i64)> = None;
        let mut sequence = Vec::new();
        for event in &mut trace.events {
            let a = activity(event).to_string();
            let time = timestamp(event)?;
            let keys: Vec<_> = event
                .attributes
                .iter()
                .filter(|(k, v)| {
                    o.blocklist.contains(k.as_ref())
                        || (k.as_ref() != "concept:name"
                            && k.as_ref() != "time:timestamp"
                            && !scalar(v))
                })
                .map(|(k, _)| k.to_string())
                .collect();
            for key in keys {
                event.attributes.remove(&key);
            }
            if o.blocklist.contains("concept:name") || o.blocklist.contains("time:timestamp") {
                return Err(PrivacyError::Options(
                    "activity and timestamp cannot be blocked",
                ));
            }
            structures.entry(a.clone()).or_insert_with(|| {
                event
                    .attributes
                    .iter()
                    .filter(|(k, _)| k.as_ref() != "concept:name" && k.as_ref() != "time:timestamp")
                    .map(|(k, _)| k.to_string())
                    .collect()
            });
            for (key, v) in event.attributes.iter().filter(|(k, _)| {
                k.as_ref() != "concept:name"
                    && k.as_ref() != "time:timestamp"
                    && k.as_ref() != "variant"
            }) {
                pools.entry(key.to_string()).or_default().push(v.clone());
            }
            if let Some((prev, prevtime)) = last {
                let delay = time - prevtime;
                if delay < 0 {
                    return Err(PrivacyError::Input(
                        "timestamps must be nondecreasing within traces".into(),
                    ));
                }
                pair_delays
                    .entry((prev, a.clone()))
                    .or_default()
                    .push(delay);
                delays.push(delay);
                times.push(time);
            }
            last = Some((a.clone(), time));
            sequence.push(a);
        }
        labels.push(sequence);
    }
    if times.is_empty() {
        return Err(PrivacyError::Input(
            "at least one adjacent timestamp pair is required".into(),
        ));
    }
    for (key, values) in &pools {
        let ty = values[0].type_name();
        if values.iter().any(|v| v.type_name() != ty) {
            return Err(PrivacyError::Input(format!(
                "mixed attribute types for {key}"
            )));
        }
    }
    let matches = matching::assign(query, &labels);
    let mut output = EventLog::default();
    let min = *times.iter().min().expect("timestamps");
    let max = *times.iter().max().expect("timestamps");
    for (id, sequence) in query.iter().enumerate() {
        let mut stacks: BTreeMap<String, VecDeque<Event>> = BTreeMap::new();
        if let Some(i) = matches[id] {
            for event in &source.traces[i].events {
                stacks
                    .entry(activity(event).to_string())
                    .or_default()
                    .push_back(event.clone());
            }
        }
        let mut trace = Trace::with_case_id(id.to_string());
        let mut last: Option<(String, i64)> = None;
        for a in sequence {
            let reused = stacks.get_mut(a).and_then(VecDeque::pop_front);
            let mut event = reused.clone().unwrap_or_default();
            event.insert("concept:name", a.as_str());
            if reused.is_none() {
                for key in &structures[a] {
                    if let Some(values) = pools.get(key) {
                        event.insert(
                            key.as_str(),
                            values[rng.random_range(0..values.len())].clone(),
                        );
                    }
                }
            }
            let old = reused.as_ref().map(timestamp).transpose()?;
            let time = if let Some(old) = old.filter(|v| last.as_ref().is_none_or(|(_, t)| v >= t))
            {
                old
            } else if let Some((prev, prevtime)) = &last {
                let pool = pair_delays
                    .get(&(prev.clone(), a.clone()))
                    .unwrap_or(&delays);
                prevtime.saturating_add(pool[rng.random_range(0..pool.len())])
            } else {
                times[rng.random_range(0..times.len())]
            };
            set_timestamp(&mut event, time)?;
            last = Some((a.clone(), time));
            trace.events.push(event);
        }
        for event in &mut trace.events {
            for (key, values) in &pools {
                let Some(value) = event.get(key).cloned() else {
                    continue;
                };
                let noisy = match value {
                    AttributeValue::Bool(b) => {
                        let keep = 1. / (1. + (-o.epsilon).exp());
                        AttributeValue::Bool(if rng.random::<f64>() < keep { b } else { !b })
                    }
                    AttributeValue::Int(v) => {
                        let lo = values
                            .iter()
                            .filter_map(AttributeValue::as_i64)
                            .min()
                            .expect("integer domain");
                        let hi = values
                            .iter()
                            .filter_map(AttributeValue::as_i64)
                            .max()
                            .expect("integer domain");
                        AttributeValue::Int(
                            mechanisms::bounded(v as f64, lo as f64, hi as f64, o.epsilon, rng)
                                .round_ties_even() as i64,
                        )
                    }
                    AttributeValue::Float(v) => {
                        let lo = values
                            .iter()
                            .filter_map(AttributeValue::as_f64)
                            .fold(f64::INFINITY, f64::min);
                        let hi = values
                            .iter()
                            .filter_map(AttributeValue::as_f64)
                            .fold(f64::NEG_INFINITY, f64::max);
                        AttributeValue::Float(mechanisms::bounded(v, lo, hi, o.epsilon, rng))
                    }
                    AttributeValue::String(v) => {
                        let mut domain: Vec<_> =
                            values.iter().filter_map(AttributeValue::as_str).collect();
                        domain.sort();
                        domain.dedup();
                        let index = domain
                            .iter()
                            .position(|x| *x == v.as_ref())
                            .expect("categorical domain");
                        AttributeValue::string(
                            domain[mechanisms::categorical(index, domain.len(), o.epsilon, rng)],
                        )
                    }
                    _ => continue,
                };
                event.insert(key.as_str(), noisy);
            }
        }
        // PRIPEL shifts the whole trace and retains its resolved inter-event gaps.
        // Recalibrate to each shift interval instead of reusing a cached scale
        // from the first trace as pm4py's LaplaceBoundedDomain does.
        // Sensitivity is the admissible interval width, smaller than pm4py's
        // whole-log range: this can yield less noise for the same epsilon.
        let begin = timestamp(&trace.events[0])?;
        let end = timestamp(trace.events.last().expect("nonempty query"))?;
        let lo = (min - begin) as f64 / 1e6;
        let mut hi = (max - end) as f64 / 1e6;
        if lo >= hi {
            hi = ((max - begin) as f64 / 1e6).abs();
        }
        let shift = mechanisms::bounded(0., lo, hi, o.epsilon, rng) * 1e6;
        for event in &mut trace.events {
            let time = timestamp(event)?;
            set_timestamp(event, (time as f64 + shift).round_ties_even() as i64)?;
        }
        output.traces.push(trace);
    }
    Ok(output)
}
