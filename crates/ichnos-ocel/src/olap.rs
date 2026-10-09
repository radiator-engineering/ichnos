//! Object-centric drill-down, roll-up, unfold and fold.
use crate::{ObjectField, Ocel, TransformationError};
use ichnos_core::AttributeValue;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn value(object: &crate::OcelObject, field: ObjectField<'_>) -> Option<AttributeValue> {
    match field {
        ObjectField::Id => Some(AttributeValue::String(object.id.clone())),
        ObjectField::Type => Some(AttributeValue::String(object.object_type.clone())),
        ObjectField::Attribute(k) => object.attributes.get(k).cloned(),
    }
}
fn validate(log: &Ocel, field: ObjectField<'_>) -> Result<(), TransformationError> {
    if let ObjectField::Attribute(k) = field
        && !log.objects.iter().any(|o| o.attributes.contains_key(k))
    {
        return Err(TransformationError::MissingAttribute(k.into()));
    }
    Ok(())
}
fn update_changes(log: &mut Ocel) {
    // pandas dict(zip(...)) takes the last duplicate object row.
    let types: BTreeMap<_, _> = log
        .objects
        .iter()
        .map(|o| (o.id.clone(), o.object_type.clone()))
        .collect();
    for c in &mut log.object_changes {
        if let Some(t) = types.get(&c.object) {
            c.object_type = t.clone();
        }
    }
}
/// Splits matching object types into `(parent, value)` using static attributes.
/// Missing, empty and NaN values keep their type. Object changes receive the
/// new type but do not drive it. Missing type or attribute columns error.
pub fn ocel_drill_down(
    log: &Ocel,
    object_type: &str,
    attribute: ObjectField<'_>,
) -> Result<Ocel, TransformationError> {
    if !log.objects.iter().any(|o| &*o.object_type == object_type) {
        return Err(TransformationError::MissingObjectType(object_type.into()));
    }
    validate(log, attribute)?;
    let mut result = log.clone();
    let mut changed = false;
    for o in &mut result.objects {
        if &*o.object_type == object_type
            && let Some(v) = value(o, attribute)
        {
            if matches!(v.plain(),AttributeValue::Float(n) if n.is_nan()) {
                continue;
            }
            let text = v.to_string();
            if !text.is_empty() {
                o.object_type = format!("({object_type}, {text})").into();
                changed = true;
            }
        }
    }
    if changed {
        update_changes(&mut result);
    }
    Ok(result)
}
/// Collapses `(parent, value)` type names back to the parent. The optional
/// attribute is validated for API symmetry but does not affect matching.
pub fn ocel_roll_up(
    log: &Ocel,
    object_type: &str,
    attribute: Option<ObjectField<'_>>,
) -> Result<Ocel, TransformationError> {
    if let Some(field) = attribute {
        validate(log, field)?;
    }
    let prefix = format!("({object_type}, ");
    let mut result = log.clone();
    let mut changed = false;
    for o in &mut result.objects {
        if o.object_type.starts_with(&prefix) && o.object_type.ends_with(')') {
            o.object_type = object_type.into();
            changed = true;
        }
    }
    if changed {
        update_changes(&mut result);
    }
    Ok(result)
}
/// Unfolds an activity to `(activity, object_type)` when at least one relation
/// matches the type and optional qualifiers. `None` accepts all qualifiers;
/// `Some(&[])` accepts none, and `None` inside a slice matches missing qualifiers.
pub fn ocel_unfold(
    log: &Ocel,
    event_type: &str,
    object_type: &str,
    qualifiers: Option<&[Option<&str>]>,
) -> Result<Ocel, TransformationError> {
    let events = log.event_index();
    let objects = log.object_index();
    let mut matched = BTreeSet::new();
    for r in &log.relations {
        let e = &log.events[*events
            .get(&*r.event)
            .ok_or_else(|| TransformationError::MissingEvent(r.event.to_string()))?];
        let o = &log.objects[*objects
            .get(&*r.object)
            .ok_or_else(|| TransformationError::MissingObject(r.object.to_string()))?];
        if &*e.activity == event_type
            && &*o.object_type == object_type
            && qualifiers.is_none_or(|qs| qs.contains(&r.qualifier.as_deref()))
        {
            matched.insert(r.event.clone());
        }
    }
    let activity: Arc<str> = format!("({event_type}, {object_type})").into();
    let mut result = log.clone();
    for e in &mut result.events {
        if matched.contains(&e.id) {
            e.activity = activity.clone();
        }
    }
    Ok(result)
}
/// Reverses the exact `(activity, object_type)` encoding used by unfold.
pub fn ocel_fold(log: &Ocel, event_type: &str, object_type: &str) -> Ocel {
    let activity = format!("({event_type}, {object_type})");
    let mut result = log.clone();
    for e in &mut result.events {
        if *e.activity == activity {
            e.activity = event_type.into();
        }
    }
    result
}
