//! Case service, sojourn, waiting, arrival, and finish enrichment.
use ichnos_core::{AttributeValue, EventLog};
use std::collections::BTreeMap;

/// Attributes read and written by case enrichment.
#[derive(Debug, Clone)]
pub struct CaseTimeOptions {
    /// Trace attribute identifying the case.
    pub case_id: String,
    /// Start timestamp attribute; defaults to the completion timestamp.
    pub start_timestamp: String,
    /// Completion timestamp attribute.
    pub timestamp: String,
    /// Per-event completion minus start duration.
    pub event_duration: String,
    /// Sum of event durations in a case.
    pub service: String,
    /// Last completion minus first start in a case.
    pub sojourn: String,
    /// Sojourn minus service.
    pub waiting: String,
    /// Elapsed seconds since the preceding case start.
    pub arrival: String,
    /// Elapsed seconds since the preceding case finish.
    pub finish: String,
}
impl Default for CaseTimeOptions {
    fn default() -> Self {
        Self {
            case_id: "concept:name".into(),
            start_timestamp: "time:timestamp".into(),
            timestamp: "time:timestamp".into(),
            event_duration: "@@diff_start_end".into(),
            service: "@@service_time".into(),
            sojourn: "@@sojourn_time".into(),
            waiting: "@@waiting_time".into(),
            arrival: "@@arrival_rate".into(),
            finish: "@@finish_rate".into(),
        }
    }
}
/// Missing or incompatible attributes in an enrichment input.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A trace has no scalar case identifier.
    #[error("trace {0} has no supported case identifier")]
    CaseId(usize),
    /// An event has no timestamp of the required type.
    #[error("trace {trace}, event {event}: attribute {key} must be a date")]
    Timestamp {
        /// Trace index.
        trace: usize,
        /// Event index.
        event: usize,
        /// Attribute name.
        key: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CaseId {
    String(String),
    Int(i64),
}
struct Case {
    start: f64,
    end: f64,
    service: f64,
}
type CaseTimes = (Vec<Option<CaseId>>, BTreeMap<CaseId, Case>, Vec<Vec<f64>>);
fn times(log: &EventLog, o: &CaseTimeOptions) -> Result<CaseTimes, Error> {
    let mut ids = Vec::new();
    let mut cases: BTreeMap<CaseId, Case> = BTreeMap::new();
    let mut durations = Vec::new();
    for (ti, trace) in log.traces.iter().enumerate() {
        if trace.events.is_empty() {
            ids.push(None);
            durations.push(Vec::new());
            continue;
        }
        let id = match trace.attributes.get(&o.case_id).map(AttributeValue::plain) {
            Some(AttributeValue::String(s) | AttributeValue::Id(s)) => {
                CaseId::String(s.to_string())
            }
            Some(AttributeValue::Int(n)) => CaseId::Int(*n),
            _ => return Err(Error::CaseId(ti)),
        };
        let mut ds = Vec::new();
        for (ei, event) in trace.events.iter().enumerate() {
            let timestamp = |key: &str| -> Result<_, Error> {
                let date = event
                    .get(key)
                    .and_then(AttributeValue::as_date)
                    .ok_or_else(|| Error::Timestamp {
                        trace: ti,
                        event: ei,
                        key: key.into(),
                    })?;
                Ok(date)
            };
            let start_date = timestamp(&o.start_timestamp)?;
            let end_date = timestamp(&o.timestamp)?;
            let start = start_date.timestamp() as f64
                + f64::from(start_date.timestamp_subsec_micros()) / 1e6;
            let end =
                end_date.timestamp() as f64 + f64::from(end_date.timestamp_subsec_micros()) / 1e6;
            // Convert the checked dates to a duration before epoch arithmetic.
            let duration = end_date
                .signed_duration_since(start_date)
                .num_microseconds()
                .map(|n| n as f64 / 1e6)
                .unwrap_or(end - start);
            let case = cases.entry(id.clone()).or_insert(Case {
                start,
                end,
                service: 0.0,
            });
            case.start = case.start.min(start);
            case.end = case.end.max(end);
            case.service += duration;
            ds.push(duration);
        }
        ids.push(Some(id));
        durations.push(ds);
    }
    Ok((ids, cases, durations))
}
/// Enrich every event with case service, sojourn, and waiting seconds.
/// Repeated trace identifiers are grouped as one case. Durations are summed,
/// including overlaps and negative intervals. Input and metadata are retained.
pub fn insert_case_service_waiting_time(
    log: &EventLog,
    o: &CaseTimeOptions,
) -> Result<EventLog, Error> {
    let (ids, cases, durations) = times(log, o)?;
    let mut out = log.clone();
    for (i, trace) in out.traces.iter_mut().enumerate() {
        if let Some(id) = &ids[i] {
            let c = &cases[id];
            let sojourn = c.end - c.start;
            for (j, event) in trace.events.iter_mut().enumerate() {
                event.insert(o.event_duration.clone(), durations[i][j]);
                event.insert(o.service.clone(), c.service);
                event.insert(o.sojourn.clone(), sojourn);
                event.insert(o.waiting.clone(), sojourn - c.service);
            }
        }
    }
    Ok(out)
}
/// Enrich events with seconds since the preceding case start and finish.
/// The first case in each ordering gets zero. Ties use typed case identifiers.
pub fn insert_case_arrival_finish_rate(
    log: &EventLog,
    o: &CaseTimeOptions,
) -> Result<EventLog, Error> {
    let (ids, cases, _) = times(log, o)?;
    let gaps = |finish: bool| {
        let mut ordered: Vec<_> = cases
            .iter()
            .map(|(id, c)| (if finish { c.end } else { c.start }, id))
            .collect();
        ordered.sort_by(|(a, ai), (b, bi)| a.total_cmp(b).then_with(|| ai.cmp(bi)));
        let mut result = BTreeMap::new();
        let mut previous = None;
        for (t, id) in ordered {
            result.insert(id.clone(), previous.map_or(0.0, |p| t - p));
            previous = Some(t);
        }
        result
    };
    let arrivals = gaps(false);
    let finishes = gaps(true);
    let mut out = log.clone();
    for (i, trace) in out.traces.iter_mut().enumerate() {
        if let Some(id) = &ids[i] {
            for event in &mut trace.events {
                event.insert(o.arrival.clone(), arrivals[id]);
                event.insert(o.finish.clone(), finishes[id]);
            }
        }
    }
    Ok(out)
}
