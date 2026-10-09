//! pm4py's default OCEL column names and JSON keys
//! (`objects/ocel/constants.py`), for readers and writers.

/// The event id column.
pub const EVENT_ID: &str = "ocel:eid";
/// The activity column.
pub const EVENT_ACTIVITY: &str = "ocel:activity";
/// The timestamp column.
pub const EVENT_TIMESTAMP: &str = "ocel:timestamp";
/// The object id column.
pub const OBJECT_ID: &str = "ocel:oid";
/// The object type column.
pub const OBJECT_TYPE: &str = "ocel:type";
/// The prefix of the per-type columns of the extended table.
pub const OBJECT_TYPE_PREFIX_EXTENDED: &str = "ocel:type:";
/// The qualifier column.
pub const QUALIFIER: &str = "ocel:qualifier";
/// The changed-attribute column of the object changes.
pub const CHANGED_FIELD: &str = "ocel:field";

/// The prefix of OCEL keys.
pub const PREFIX: &str = "ocel:";
/// The events key of OCEL JSON.
pub const EVENTS_KEY: &str = "ocel:events";
/// The objects key of OCEL JSON.
pub const OBJECTS_KEY: &str = "ocel:objects";
/// The id key of OCEL JSON.
pub const ID_KEY: &str = "ocel:id";
/// The related-objects key of an OCEL 1.0 JSON event.
pub const OMAP_KEY: &str = "ocel:omap";
/// The qualified related-objects key of an OCEL 2.0 JSON event.
pub const TYPED_OMAP_KEY: &str = "ocel:typedOmap";
/// The object-to-object relations key of an OCEL 2.0 JSON object.
pub const O2O_KEY: &str = "ocel:o2o";
/// The event attributes key of OCEL JSON.
pub const VMAP_KEY: &str = "ocel:vmap";
/// The object attributes key of OCEL JSON.
pub const OVMAP_KEY: &str = "ocel:ovmap";
/// The object changes key of OCEL JSON.
pub const OBJECT_CHANGES_KEY: &str = "ocel:objectChanges";
/// The event types key of OCEL 2.0 JSON.
pub const EVENT_TYPES_KEY: &str = "ocel:eventTypes";
/// The object types key of OCEL 2.0 JSON.
pub const OBJECT_TYPES_KEY: &str = "ocel:objectTypes";
/// The global log key of OCEL 1.0.
pub const GLOBAL_LOG: &str = "ocel:global-log";
/// The attribute names key of the global log.
pub const GLOBAL_LOG_ATTRIBUTE_NAMES: &str = "ocel:attribute-names";
/// The object types key of the global log.
pub const GLOBAL_LOG_OBJECT_TYPES: &str = "ocel:object-types";
/// The version key of the global log.
pub const GLOBAL_LOG_VERSION: &str = "ocel:version";
/// The ordering key of the global log.
pub const GLOBAL_LOG_ORDERING: &str = "ocel:ordering";
/// The global event key of OCEL 1.0.
pub const GLOBAL_EVENT: &str = "ocel:global-event";
/// The global object key of OCEL 1.0.
pub const GLOBAL_OBJECT: &str = "ocel:global-object";
/// The default ordering of the global log.
pub const DEFAULT_ORDERING: &str = "timestamp";
/// The OCEL version pm4py writes.
pub const CURRENT_VERSION: &str = "1.0";
