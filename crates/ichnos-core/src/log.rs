//! Events, traces, event logs and event streams.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use rustc_hash::FxHashMap;

use crate::attribute::{AttributeValue, Attributes};
use crate::error::{Error, Position, Result};
use crate::keys::{self, EventKeys};

/// One event: a map of attributes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Event {
    /// The event's attributes, in insertion order.
    pub attributes: Attributes,
}

impl Event {
    /// An event with no attributes.
    pub fn new() -> Self {
        Self::default()
    }

    /// The value of attribute `key`.
    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }

    /// Sets attribute `key`. Returns the old value.
    pub fn insert<K, V>(&mut self, key: K, value: V) -> Option<AttributeValue>
    where
        K: AsRef<str> + Into<Arc<str>>,
        V: Into<AttributeValue>,
    {
        self.attributes.insert(key, value)
    }
}

impl From<Attributes> for Event {
    fn from(attributes: Attributes) -> Self {
        Self { attributes }
    }
}

impl<K, V> FromIterator<(K, V)> for Event
where
    K: AsRef<str> + Into<Arc<str>>,
    V: Into<AttributeValue>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            attributes: iter.into_iter().collect(),
        }
    }
}

/// One case: trace attributes and an ordered list of events.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trace {
    /// Trace attributes. The case ID is `concept:name`.
    pub attributes: Attributes,
    /// The events, in log order.
    pub events: Vec<Event>,
}

impl Trace {
    /// An empty trace with no attributes.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty trace whose `concept:name` is `case_id`.
    pub fn with_case_id(case_id: impl Into<AttributeValue>) -> Self {
        let mut trace = Self::new();
        trace.attributes.insert(keys::CONCEPT_NAME, case_id);
        trace
    }

    /// The case ID: the trace attribute `concept:name`.
    pub fn case_id(&self) -> Option<&AttributeValue> {
        self.attributes.get(keys::CONCEPT_NAME)
    }

    /// The number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the trace has no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Iterates over the events.
    pub fn iter(&self) -> std::slice::Iter<'_, Event> {
        self.events.iter()
    }
}

impl<'a> IntoIterator for &'a Trace {
    type Item = &'a Event;
    type IntoIter = std::slice::Iter<'a, Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.iter()
    }
}

/// An XES extension declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Extension {
    /// Extension name, for example `Concept`.
    pub name: String,
    /// Key prefix, for example `concept`.
    pub prefix: String,
    /// Definition URI.
    pub uri: String,
}

/// The standard XES extensions that pm4py knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XesExtension {
    /// `artifactlifecycle`.
    ArtifactLifecycle,
    /// `concept`.
    Concept,
    /// `cost`.
    Cost,
    /// `identity`.
    Identity,
    /// `lifecycle`.
    Lifecycle,
    /// `micro`.
    Micro,
    /// `org`.
    Organizational,
    /// `semantic`.
    Semantic,
    /// `swcomm`.
    SoftwareCommunication,
    /// `swevent`.
    SoftwareEvent,
    /// `swtelemetry`.
    SoftwareTelemetry,
    /// `time`.
    Time,
}

impl XesExtension {
    /// Every standard extension, in pm4py's declaration order.
    pub const ALL: [Self; 12] = [
        Self::ArtifactLifecycle,
        Self::Concept,
        Self::Cost,
        Self::Identity,
        Self::Lifecycle,
        Self::Micro,
        Self::Organizational,
        Self::Semantic,
        Self::SoftwareCommunication,
        Self::SoftwareEvent,
        Self::SoftwareTelemetry,
        Self::Time,
    ];

    fn parts(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::ArtifactLifecycle => (
                "ArtifactLifecycle",
                "artifactlifecycle",
                "http://www.xes-standard.org/artifactlifecycle.xesext",
            ),
            Self::Concept => (
                "Concept",
                "concept",
                "http://www.xes-standard.org/concept.xesext",
            ),
            Self::Cost => ("Cost", "cost", "http://www.xes-standard.org/cost.xesext"),
            Self::Identity => (
                "Identity",
                "identity",
                "http://www.xes-standard.org/identity.xesext",
            ),
            Self::Lifecycle => (
                "Lifecycle",
                "lifecycle",
                "http://www.xes-standard.org/lifecycle.xesext",
            ),
            Self::Micro => ("Micro", "micro", "http://www.xes-standard.org/micro.xesext"),
            Self::Organizational => (
                "Organizational",
                "org",
                "http://www.xes-standard.org/org.xesext",
            ),
            Self::Semantic => (
                "Semantic",
                "semantic",
                "http://www.xes-standard.org/semantic.xesext",
            ),
            Self::SoftwareCommunication => (
                "Software Communication",
                "swcomm",
                "http://www.xes-standard.org/swcomm.xesext",
            ),
            Self::SoftwareEvent => (
                "Software Event",
                "swevent",
                "http://www.xes-standard.org/swevent.xesext",
            ),
            Self::SoftwareTelemetry => (
                "Software Telemetry",
                "swtelemetry",
                "http://www.xes-standard.org/swtelemetry.xesext",
            ),
            Self::Time => ("Time", "time", "http://www.xes-standard.org/time.xesext"),
        }
    }

    /// The extension name, for example `Organizational`.
    pub fn name(self) -> &'static str {
        self.parts().0
    }

    /// The key prefix, for example `org`.
    pub fn prefix(self) -> &'static str {
        self.parts().1
    }

    /// The definition URI.
    pub fn uri(self) -> &'static str {
        self.parts().2
    }

    /// The standard extension with this key prefix.
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.prefix() == prefix)
    }
}

impl From<XesExtension> for Extension {
    fn from(ext: XesExtension) -> Self {
        Self {
            name: ext.name().to_owned(),
            prefix: ext.prefix().to_owned(),
            uri: ext.uri().to_owned(),
        }
    }
}

/// A named XES classifier: the event attributes whose values, joined with
/// `+`, identify an event class.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Classifier {
    /// Classifier name, for example `Activity classifier`.
    pub name: String,
    /// The event attribute keys, in order.
    pub keys: Vec<String>,
}

/// XES global attributes: the attributes every trace or event is declared to
/// carry, with their default values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Globals {
    /// Globals with scope `trace`.
    pub trace: Attributes,
    /// Globals with scope `event`.
    pub event: Attributes,
}

/// An event log: log-level XES metadata and a list of traces.
///
/// This is the type every ichnos algorithm takes, as `&EventLog`. The Arrow
/// `RecordBatch` view ([`EventLog::to_arrow`], [`EventLog::from_arrow`]) is
/// for I/O and interchange.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventLog {
    /// Log attributes.
    pub attributes: Attributes,
    /// Declared extensions, in declaration order.
    pub extensions: Vec<Extension>,
    /// Global attributes.
    pub globals: Globals,
    /// Declared classifiers, in declaration order.
    pub classifiers: Vec<Classifier>,
    /// The traces.
    pub traces: Vec<Trace>,
}

impl EventLog {
    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// A log with these traces and no metadata.
    pub fn from_traces(traces: Vec<Trace>) -> Self {
        Self {
            traces,
            ..Self::default()
        }
    }

    /// The number of traces.
    pub fn len(&self) -> usize {
        self.traces.len()
    }

    /// Whether the log has no traces.
    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    /// The total number of events.
    pub fn num_events(&self) -> usize {
        self.traces.iter().map(Trace::len).sum()
    }

    /// Iterates over the traces.
    pub fn iter(&self) -> std::slice::Iter<'_, Trace> {
        self.traces.iter()
    }

    /// Iterates over all events, trace by trace.
    pub fn events(&self) -> impl Iterator<Item = &Event> {
        self.traces.iter().flat_map(|t| t.events.iter())
    }

    /// The classifier named `name`.
    pub fn classifier(&self, name: &str) -> Option<&Classifier> {
        self.classifiers.iter().find(|c| c.name == name)
    }

    /// Builds a log from traces written as strings, for example
    /// `["A,B,C", "A,C"]`. Port of pm4py's `parse_event_log_string`.
    ///
    /// Trace `i` gets case ID `"i"`. Events get the activity and a timestamp.
    /// Timestamps start at 10,000,000 seconds after the Unix epoch, in UTC,
    /// and grow by one second per event across the whole log.
    pub fn from_trace_strings<'a>(
        traces: impl IntoIterator<Item = &'a str>,
        separator: &str,
        keys: &EventKeys,
    ) -> Self {
        let activity: Arc<str> = keys.activity.as_str().into();
        let timestamp: Arc<str> = keys.timestamp.as_str().into();
        let mut seconds = 10_000_000_i64;
        let traces = traces
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                let mut trace = Trace::with_case_id(index.to_string());
                for name in text.split(separator) {
                    let mut event = Event::new();
                    event.insert(activity.clone(), name);
                    let at = DateTime::<Utc>::from_timestamp(seconds, 0).expect("in range");
                    event.insert(timestamp.clone(), at);
                    trace.events.push(event);
                    seconds += 1;
                }
                trace
            })
            .collect();
        Self::from_traces(traces)
    }

    /// The value of event attribute `key` for every event, trace by trace.
    /// Port of pm4py's `project_on_event_attribute`; a missing attribute gives
    /// `None`.
    pub fn project(&self, key: &str) -> Vec<Vec<Option<&AttributeValue>>> {
        self.traces
            .iter()
            .map(|t| t.events.iter().map(|e| e.get(key)).collect())
            .collect()
    }

    /// Writes the classifier value of every event to `target`: the values of
    /// `attribute_keys`, formatted as Python's `str()` would and joined with
    /// `+`. Port of pm4py's `set_classifier` and
    /// `insert_activity_classifier_attribute`.
    ///
    /// Use `target` as the activity key afterwards. Fails without changing the
    /// log if an event lacks one of the keys.
    pub fn insert_classifier_attribute<S: AsRef<str>>(
        &mut self,
        attribute_keys: &[S],
        target: &str,
    ) -> Result<()> {
        for (t, trace) in self.traces.iter().enumerate() {
            for (e, event) in trace.events.iter().enumerate() {
                if let Some(key) = attribute_keys
                    .iter()
                    .find(|k| !event.attributes.contains_key(k.as_ref()))
                {
                    return Err(Error::MissingAttribute {
                        key: key.as_ref().to_owned(),
                        position: Position::Event { trace: t, event: e },
                    });
                }
            }
        }
        let target: Arc<str> = target.into();
        for event in self.traces.iter_mut().flat_map(|t| t.events.iter_mut()) {
            let mut value = String::new();
            for (i, key) in attribute_keys.iter().enumerate() {
                if i > 0 {
                    value.push('+');
                }
                let part = event.get(key.as_ref()).expect("checked above");
                value.push_str(&part.to_string());
            }
            event.insert(target.clone(), value);
        }
        Ok(())
    }

    /// Like [`insert_classifier_attribute`](Self::insert_classifier_attribute)
    /// with the keys of the log's classifier named `classifier`.
    pub fn insert_named_classifier_attribute(
        &mut self,
        classifier: &str,
        target: &str,
    ) -> Result<()> {
        let keys = self
            .classifier(classifier)
            .ok_or_else(|| Error::UnknownClassifier(classifier.to_owned()))?
            .keys
            .clone();
        self.insert_classifier_attribute(&keys, target)
    }

    /// Flattens the log into an event stream, cloning the events. Each event
    /// gets the trace attributes under `keys.case_prefix` (a trace attribute
    /// overwrites an event attribute of the same prefixed name). Port of
    /// pm4py's `to_event_stream` with `include_case_attributes=True`.
    pub fn to_event_stream(&self, keys: &EventKeys) -> EventStream {
        self.clone().into_event_stream(keys)
    }

    /// Like [`to_event_stream`](Self::to_event_stream), consuming the log.
    pub fn into_event_stream(self, keys: &EventKeys) -> EventStream {
        let mut events = Vec::with_capacity(self.num_events());
        for trace in self.traces {
            let prefixed: Vec<(Arc<str>, &AttributeValue)> = trace
                .attributes
                .iter()
                .map(|(k, v)| (Arc::from(format!("{}{k}", keys.case_prefix)), v))
                .collect();
            for mut event in trace.events {
                for (k, v) in &prefixed {
                    event.attributes.insert(k.clone(), (*v).clone());
                }
                events.push(event);
            }
        }
        EventStream {
            attributes: self.attributes,
            extensions: self.extensions,
            globals: self.globals,
            classifiers: self.classifiers,
            events,
        }
    }
}

impl<'a> IntoIterator for &'a EventLog {
    type Item = &'a Trace;
    type IntoIter = std::slice::Iter<'a, Trace>;

    fn into_iter(self) -> Self::IntoIter {
        self.traces.iter()
    }
}

/// A flat list of events with log-level XES metadata. Trace attributes, if
/// any, live in each event under the `case:` prefix.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventStream {
    /// Log attributes.
    pub attributes: Attributes,
    /// Declared extensions.
    pub extensions: Vec<Extension>,
    /// Global attributes.
    pub globals: Globals,
    /// Declared classifiers.
    pub classifiers: Vec<Classifier>,
    /// The events.
    pub events: Vec<Event>,
}

impl EventStream {
    /// An empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the stream has no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Groups the events into traces by `keys.case_id`. Port of pm4py's
    /// `to_event_log` conversion.
    ///
    /// Traces appear in order of their first event. Attributes starting with
    /// `keys.case_prefix` move to the trace, with the prefix removed, taken
    /// from the case's first event. A trace without `concept:name` gets the
    /// case ID as `concept:name`. Fails if an event has no case ID.
    pub fn into_event_log(self, keys: &EventKeys) -> Result<EventLog> {
        let prefix = keys.case_prefix.as_str();
        let mut index: FxHashMap<CaseKey, usize> = FxHashMap::default();
        let mut traces: Vec<Trace> = Vec::new();
        for (i, mut event) in self.events.into_iter().enumerate() {
            let position = Position::StreamEvent(i);
            let case_id = event
                .get(&keys.case_id)
                .ok_or_else(|| Error::MissingAttribute {
                    key: keys.case_id.clone(),
                    position,
                })?;
            let case_key = CaseKey::new(case_id).ok_or_else(|| Error::AttributeType {
                key: keys.case_id.clone(),
                position,
                expected: "scalar",
                found: case_id.type_name(),
            })?;
            let slot = *index.entry(case_key).or_insert_with(|| {
                let mut trace = Trace::new();
                for (k, v) in &event.attributes {
                    if let Some(stripped) = k.strip_prefix(prefix) {
                        trace.attributes.insert(stripped, v.clone());
                    }
                }
                if !trace.attributes.contains_key(keys::CONCEPT_NAME) {
                    trace.attributes.insert(keys::CONCEPT_NAME, case_id.clone());
                }
                traces.push(trace);
                traces.len() - 1
            });
            event.attributes.retain(|k, _| !k.starts_with(prefix));
            traces[slot].events.push(event);
        }
        Ok(EventLog {
            attributes: self.attributes,
            extensions: self.extensions,
            globals: self.globals,
            classifiers: self.classifiers,
            traces,
        })
    }
}

impl<'a> IntoIterator for &'a EventStream {
    type Item = &'a Event;
    type IntoIter = std::slice::Iter<'a, Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.iter()
    }
}

/// A hashable case ID. Strings and IDs with the same text are one case, and
/// so are an int and an integral float, as with Python dict keys.
#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum CaseKey {
    Text(Arc<str>),
    Int(i64),
    Float(u64),
    Bool(bool),
    Date(DateTime<Utc>),
}

impl CaseKey {
    pub(crate) fn new(value: &AttributeValue) -> Option<Self> {
        Some(match value.plain() {
            AttributeValue::String(s) | AttributeValue::Id(s) => Self::Text(s.clone()),
            AttributeValue::Int(v) => Self::Int(*v),
            AttributeValue::Float(v) if v.fract() == 0.0 && v.abs() < 9.0e18 => {
                Self::Int(*v as i64)
            }
            AttributeValue::Float(v) => Self::Float(v.to_bits()),
            AttributeValue::Bool(v) => Self::Bool(*v),
            AttributeValue::Date(d) => Self::Date(d.to_utc()),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(e: &Event) -> &str {
        e.get("concept:name")
            .and_then(AttributeValue::as_str)
            .unwrap()
    }

    #[test]
    fn from_trace_strings_matches_pm4py() {
        let log = EventLog::from_trace_strings(["A,B,C", "A,C"], ",", &EventKeys::default());
        assert_eq!(log.len(), 2);
        assert_eq!(log.num_events(), 5);
        assert_eq!(log.traces[1].case_id().and_then(|v| v.as_str()), Some("1"));
        let acts: Vec<&str> = log.traces[0].iter().map(act).collect();
        assert_eq!(acts, ["A", "B", "C"]);
        let last = log.traces[1].events[1].get("time:timestamp").unwrap();
        assert_eq!(last.as_date().unwrap().timestamp(), 10_000_004);
        assert_eq!(last.to_string(), "1970-04-26 17:46:44+00:00");
    }

    #[test]
    fn stream_round_trip_moves_case_attributes() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A,B", "C"], ",", &keys);
        log.traces[0].attributes.insert("region", "north");
        let stream = log.to_event_stream(&keys);
        assert_eq!(stream.len(), 3);
        let first = &stream.events[0];
        assert_eq!(
            first.get("case:concept:name").and_then(|v| v.as_str()),
            Some("0")
        );
        assert_eq!(
            first.get("case:region").and_then(|v| v.as_str()),
            Some("north")
        );
        let back = stream.into_event_log(&keys).unwrap();
        assert_eq!(back, log);
    }

    #[test]
    fn into_event_log_groups_by_first_appearance() {
        let keys = EventKeys::default().with_case_id("case");
        let stream = EventStream {
            events: vec![
                Event::from_iter([
                    ("case", AttributeValue::from(2)),
                    ("concept:name", "a".into()),
                ]),
                Event::from_iter([
                    ("case", AttributeValue::from(1)),
                    ("concept:name", "b".into()),
                ]),
                Event::from_iter([
                    ("case", AttributeValue::from(2.0)),
                    ("concept:name", "c".into()),
                ]),
            ],
            ..EventStream::default()
        };
        let log = stream.into_event_log(&keys).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log.traces[0].case_id(), Some(&AttributeValue::Int(2)));
        assert_eq!(log.traces[0].len(), 2);
        // A case ID column without the prefix stays an event attribute.
        assert!(log.traces[0].events[0].get("case").is_some());
    }

    #[test]
    fn into_event_log_reports_missing_case_id() {
        let stream = EventStream {
            events: vec![Event::from_iter([("concept:name", "a")])],
            ..EventStream::default()
        };
        let err = stream.into_event_log(&EventKeys::default()).unwrap_err();
        assert!(matches!(
            err,
            Error::MissingAttribute {
                position: Position::StreamEvent(0),
                ..
            }
        ));
    }

    #[test]
    fn classifier_joins_values() {
        let mut log = EventLog::from_trace_strings(["A,B"], ",", &EventKeys::default());
        for e in &mut log.traces[0].events {
            e.insert("lifecycle:transition", "complete");
        }
        log.classifiers.push(Classifier {
            name: "Activity classifier".into(),
            keys: vec!["concept:name".into(), "lifecycle:transition".into()],
        });
        log.insert_named_classifier_attribute("Activity classifier", "@@classifier")
            .unwrap();
        let v = log.traces[0].events[1].get("@@classifier").unwrap();
        assert_eq!(v.as_str(), Some("B+complete"));
        assert!(matches!(
            log.insert_classifier_attribute(&["missing"], "x"),
            Err(Error::MissingAttribute { .. })
        ));
        assert!(log.traces[0].events[0].get("x").is_none());
        assert!(matches!(
            log.insert_named_classifier_attribute("nope", "x"),
            Err(Error::UnknownClassifier(_))
        ));
    }

    #[test]
    fn project_returns_values_per_trace() {
        let log = EventLog::from_trace_strings(["A,B", "C"], ",", &EventKeys::default());
        let projected = log.project("concept:name");
        assert_eq!(projected.len(), 2);
        assert_eq!(projected[0][1].and_then(|v| v.as_str()), Some("B"));
        assert!(log.project("missing")[1][0].is_none());
    }

    #[test]
    fn standard_extensions() {
        assert_eq!(
            XesExtension::from_prefix("org"),
            Some(XesExtension::Organizational)
        );
        let ext = Extension::from(XesExtension::Time);
        assert_eq!(ext.uri, "http://www.xes-standard.org/time.xesext");
    }
}
