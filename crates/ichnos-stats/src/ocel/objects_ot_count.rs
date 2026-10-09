//! Relation counts per event and object type.
use super::*;

/// Count relation rows, including duplicate event/object pairs, per event and type.
/// Events without relations and object types without related objects are absent.
pub fn get_objects_ot_count(log: &Ocel) -> Result<BTreeMap<Id, BTreeMap<Id, usize>>> {
    let mut result: BTreeMap<Id, BTreeMap<Id, usize>> = BTreeMap::new();
    for (row, t) in relation_rows(log)? {
        *result.entry(row.event).or_default().entry(t).or_default() += 1;
    }
    Ok(result)
}
