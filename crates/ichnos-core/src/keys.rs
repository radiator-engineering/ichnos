//! Standard attribute keys and the [`EventKeys`] struct that every ichnos crate accepts.

/// XES `concept:name`: the activity of an event, the case ID of a trace.
pub const CONCEPT_NAME: &str = "concept:name";
/// XES `concept:instance`.
pub const CONCEPT_INSTANCE: &str = "concept:instance";
/// XES `time:timestamp`.
pub const TIME_TIMESTAMP: &str = "time:timestamp";
/// pm4py's default start-timestamp key for interval events.
pub const START_TIMESTAMP: &str = "start_timestamp";
/// XES `org:resource`.
pub const ORG_RESOURCE: &str = "org:resource";
/// XES `org:group`.
pub const ORG_GROUP: &str = "org:group";
/// XES `lifecycle:transition`.
pub const LIFECYCLE_TRANSITION: &str = "lifecycle:transition";
/// The prefix that marks a trace attribute in a flat table.
pub const CASE_PREFIX: &str = "case:";
/// The case ID column of a flat table.
pub const CASE_CONCEPT_NAME: &str = "case:concept:name";

/// The attribute keys that algorithms read. pm4py passes these one by one as
/// `activity_key`, `timestamp_key`, `case_id_key` and so on.
///
/// `Default` gives the XES standard keys. Override single fields with the
/// `with_*` methods or struct update syntax:
///
/// ```
/// use ichnos_core::EventKeys;
/// let keys = EventKeys::default().with_activity("task");
/// assert_eq!(keys.timestamp, "time:timestamp");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventKeys {
    /// Event attribute that names the activity. Default `concept:name`.
    pub activity: String,
    /// Event attribute that holds the (completion) timestamp. Default `time:timestamp`.
    pub timestamp: String,
    /// Event attribute that holds the start timestamp. Default `start_timestamp`.
    pub start_timestamp: String,
    /// Flat-table column that identifies the case. Default `case:concept:name`.
    ///
    /// In an [`EventLog`](crate::EventLog) the case ID is the trace attribute
    /// `concept:name`. This key matters when grouping a flat table or an
    /// [`EventStream`](crate::EventStream) into traces.
    pub case_id: String,
    /// Event attribute that names the resource. Default `org:resource`.
    pub resource: String,
    /// Event attribute that holds the lifecycle transition. Default `lifecycle:transition`.
    pub transition: String,
    /// Event attribute that names the group. Default `org:group`.
    pub group: String,
    /// Prefix that marks trace attributes in a flat table. Default `case:`.
    pub case_prefix: String,
}

impl Default for EventKeys {
    fn default() -> Self {
        Self {
            activity: CONCEPT_NAME.to_owned(),
            timestamp: TIME_TIMESTAMP.to_owned(),
            start_timestamp: START_TIMESTAMP.to_owned(),
            case_id: CASE_CONCEPT_NAME.to_owned(),
            resource: ORG_RESOURCE.to_owned(),
            transition: LIFECYCLE_TRANSITION.to_owned(),
            group: ORG_GROUP.to_owned(),
            case_prefix: CASE_PREFIX.to_owned(),
        }
    }
}

impl EventKeys {
    /// Sets the activity key.
    pub fn with_activity(mut self, key: impl Into<String>) -> Self {
        self.activity = key.into();
        self
    }

    /// Sets the timestamp key.
    pub fn with_timestamp(mut self, key: impl Into<String>) -> Self {
        self.timestamp = key.into();
        self
    }

    /// Sets the case ID column.
    pub fn with_case_id(mut self, key: impl Into<String>) -> Self {
        self.case_id = key.into();
        self
    }

    /// Sets the resource key.
    pub fn with_resource(mut self, key: impl Into<String>) -> Self {
        self.resource = key.into();
        self
    }

    /// Sets the start timestamp key.
    pub fn with_start_timestamp(mut self, key: impl Into<String>) -> Self {
        self.start_timestamp = key.into();
        self
    }

    /// Sets the lifecycle transition key.
    pub fn with_transition(mut self, key: impl Into<String>) -> Self {
        self.transition = key.into();
        self
    }

    /// Sets the group key.
    pub fn with_group(mut self, key: impl Into<String>) -> Self {
        self.group = key.into();
        self
    }

    /// Sets the prefix of trace attribute columns.
    pub fn with_case_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.case_prefix = prefix.into();
        self
    }
}
