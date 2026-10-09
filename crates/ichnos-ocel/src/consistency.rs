//! The clean-up pm4py runs after reading a log: `ocel_consistency.apply`
//! and `filtering_utils.propagate_relations_filtering`.

use std::collections::HashSet;
use std::sync::Arc;

use crate::ocel::Ocel;

impl Ocel {
    /// Drops rows with an empty id, activity or type, and gives every
    /// event-to-object and object-to-object relation without a qualifier the
    /// empty qualifier (pm4py's `ocel_consistency.apply`).
    ///
    /// pm4py also drops rows whose id columns are missing; here ids are
    /// strings and cannot be missing.
    pub fn make_consistent(&mut self) {
        self.events
            .retain(|e| !e.id.is_empty() && !e.activity.is_empty());
        self.objects
            .retain(|o| !o.id.is_empty() && !o.object_type.is_empty());
        self.relations
            .retain(|r| !r.event.is_empty() && !r.object.is_empty());
        self.o2o
            .retain(|r| !r.source.is_empty() && !r.target.is_empty());
        self.e2e
            .retain(|r| !r.source.is_empty() && !r.target.is_empty());
        self.object_changes.retain(|c| !c.object.is_empty());
        let empty: Arc<str> = Arc::from("");
        for r in &mut self.relations {
            r.qualifier.get_or_insert_with(|| empty.clone());
        }
        for r in &mut self.o2o {
            r.qualifier.get_or_insert_with(|| empty.clone());
        }
    }

    /// Keeps only the events and objects that take part in a relation and
    /// exist in the log, then the rows that refer to them (pm4py's
    /// `filtering_utils.propagate_relations_filtering`).
    ///
    /// A relation is kept when both its event and its object are kept. An
    /// object-to-object or event-to-event relation is kept when both ends
    /// are kept, and an object change when its object is kept. When no
    /// event or no object is kept, every table is emptied.
    pub fn retain_related(&mut self) {
        let related_events: HashSet<&str> = self.relations.iter().map(|r| &*r.event).collect();
        let related_objects: HashSet<&str> = self.relations.iter().map(|r| &*r.object).collect();
        let events: HashSet<Arc<str>> = self
            .events
            .iter()
            .filter(|e| related_events.contains(&*e.id))
            .map(|e| e.id.clone())
            .collect();
        let objects: HashSet<Arc<str>> = self
            .objects
            .iter()
            .filter(|o| related_objects.contains(&*o.id))
            .map(|o| o.id.clone())
            .collect();
        if events.is_empty() || objects.is_empty() {
            self.events.clear();
            self.objects.clear();
            self.relations.clear();
            self.o2o.clear();
            self.e2e.clear();
            self.object_changes.clear();
            return;
        }
        self.events.retain(|e| events.contains(&e.id));
        self.objects.retain(|o| objects.contains(&o.id));
        self.relations
            .retain(|r| events.contains(&r.event) && objects.contains(&r.object));
        self.e2e
            .retain(|r| events.contains(&r.source) && events.contains(&r.target));
        self.o2o
            .retain(|r| objects.contains(&r.source) && objects.contains(&r.target));
        self.object_changes.retain(|c| objects.contains(&c.object));
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use ichnos_core::Attributes;

    use crate::{EventObject, ObjectObject, Ocel, OcelEvent, OcelObject};

    fn event(id: &str) -> OcelEvent {
        OcelEvent {
            id: id.into(),
            activity: "a".into(),
            timestamp: DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z").unwrap(),
            attributes: Attributes::default(),
        }
    }

    fn object(id: &str) -> OcelObject {
        OcelObject {
            id: id.into(),
            object_type: "t".into(),
            attributes: Attributes::default(),
        }
    }

    fn relation(e: &str, o: &str) -> EventObject {
        EventObject {
            event: e.into(),
            object: o.into(),
            qualifier: None,
        }
    }

    #[test]
    fn keeps_related_rows() {
        let mut ocel = Ocel {
            events: vec![event("e1"), event("e2")],
            objects: vec![object("o1"), object("o2")],
            relations: vec![relation("e1", "o1"), relation("e3", "o2")],
            o2o: vec![ObjectObject {
                source: "o1".into(),
                target: "o2".into(),
                qualifier: None,
            }],
            ..Ocel::default()
        };
        ocel.make_consistent();
        ocel.retain_related();
        assert_eq!(ocel.events.len(), 1);
        assert_eq!(ocel.objects.len(), 2);
        assert_eq!(ocel.relations.len(), 1);
        assert_eq!(ocel.relations[0].qualifier.as_deref(), Some(""));
        assert_eq!(ocel.o2o.len(), 1);
    }

    #[test]
    fn empties_without_relations() {
        let mut ocel = Ocel {
            events: vec![event("e1")],
            objects: vec![object("o1")],
            ..Ocel::default()
        };
        ocel.retain_related();
        assert!(ocel.events.is_empty() && ocel.objects.is_empty());
    }
}
