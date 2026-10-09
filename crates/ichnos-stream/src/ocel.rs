//! Distribute OCEL iterator-style events to per-object-type flattened streams.

use crate::{Error, Result, StreamSink};
use ichnos_core::{AttributeValue, Event, EventKeys};
use std::{collections::BTreeMap, fmt};

/// Attribute names for OCEL-to-classic stream flattening.
#[derive(Debug, Clone)]
pub struct OcelDistributorOptions {
    /// Destination activity, timestamp and case ID keys.
    pub keys: EventKeys,
    /// Source OCEL activity field.
    pub activity_key: String,
    /// Source OCEL timestamp field.
    pub timestamp_key: String,
    /// Prefix of object-type fields. Each must contain an ordered List of IDs.
    pub object_type_prefix: String,
}

impl Default for OcelDistributorOptions {
    fn default() -> Self {
        Self {
            keys: EventKeys::default(),
            activity_key: "ocel:activity".into(),
            timestamp_key: "ocel:timestamp".into(),
            object_type_prefix: "ocel:type:".into(),
        }
    }
}

/// Flatten each linked object into a classic event with that object's case ID.
///
/// Input is the canonical counterpart of an OCEL iterator row: activity and
/// timestamp attributes plus `ocel:type:<type>` List values. List child keys
/// are ignored; values retain their typed identity. Ordinary attributes are
/// copied and all object-type fields removed. Duplicate IDs and registrations
/// deliver repeatedly, as in the native distributor. Types use lexical order;
/// objects and listeners retain their input/registration order.
pub struct OcelFlatteningDistributor {
    options: OcelDistributorOptions,
    listeners: BTreeMap<String, Vec<Box<dyn StreamSink>>>,
    seen: usize,
}

impl fmt::Debug for OcelFlatteningDistributor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OcelFlatteningDistributor")
            .field("options", &self.options)
            .field("object_types", &self.listeners.keys())
            .finish()
    }
}

impl OcelFlatteningDistributor {
    /// Create a distributor. Source/destination activity and timestamp keys
    /// may be equal; unlike native rename-then-delete, those values survive.
    pub fn new(options: OcelDistributorOptions) -> Result<Self> {
        if options.object_type_prefix.is_empty()
            || options.keys.activity == options.keys.timestamp
            || options.keys.case_id == options.keys.activity
            || options.keys.case_id == options.keys.timestamp
        {
            return Err(Error::InvalidConformanceOption(
                "nonempty OCEL prefix and distinct destination keys required",
            ));
        }
        Ok(Self {
            options,
            listeners: BTreeMap::new(),
            seen: 0,
        })
    }

    /// Register an owned or shared event consumer for an object type. Use a
    /// shared LiveEventStream to retain lifecycle/result access. Repeated
    /// registration deliberately delivers the event repeatedly.
    pub fn register(
        &mut self,
        object_type: impl Into<String>,
        listener: impl StreamSink + 'static,
    ) {
        self.listeners
            .entry(object_type.into())
            .or_default()
            .push(Box::new(listener));
    }

    /// Flatten and deliver an event. Validate required fields and all object
    /// lists before delivery. Continue after sink errors and return the first;
    /// successful deliveries are not rolled back.
    pub fn append(&mut self, event: &Event) -> Result<()> {
        let index = self.seen;
        self.seen += 1;
        let required = |key: &str| {
            event.get(key).cloned().ok_or_else(|| Error::MissingField {
                key: key.into(),
                event: index,
            })
        };
        let activity = required(&self.options.activity_key)?;
        let timestamp = required(&self.options.timestamp_key)?;
        let mut objects = BTreeMap::new();
        let mut base = Event::new();
        for (key, value) in event.attributes.iter() {
            if let Some(ot) = key.strip_prefix(&self.options.object_type_prefix) {
                let AttributeValue::List(values) = value.plain() else {
                    return Err(Error::OcelObjectsType {
                        key: key.to_string(),
                        event: index,
                    });
                };
                objects.insert(ot, values);
            } else if key.as_ref() != self.options.activity_key
                && key.as_ref() != self.options.timestamp_key
            {
                base.insert(key.clone(), value.clone());
            }
        }
        base.insert(self.options.keys.activity.clone(), activity);
        base.insert(self.options.keys.timestamp.clone(), timestamp);
        let mut first_error = None;
        for (ot, ids) in objects {
            if let Some(listeners) = self.listeners.get_mut(ot) {
                for (_, id) in ids {
                    let mut flattened = base.clone();
                    flattened.insert(self.options.keys.case_id.clone(), id.clone());
                    for listener in listeners.iter_mut() {
                        if let Err(error) = listener.push(&flattened) {
                            first_error.get_or_insert(error);
                        }
                    }
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

impl Default for OcelFlatteningDistributor {
    fn default() -> Self {
        Self::new(OcelDistributorOptions::default()).expect("default keys are valid")
    }
}

impl StreamSink for OcelFlatteningDistributor {
    fn push(&mut self, event: &Event) -> Result<()> {
        self.append(event)
    }
}
