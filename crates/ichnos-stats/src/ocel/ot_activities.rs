//! Activities associated with each object type.
use super::*;

/// Distinct related activities per type; unrelated types are absent.
pub fn get_object_type_activities(log: &Ocel) -> Result<BTreeMap<Id, BTreeSet<Id>>> {
    let mut result: BTreeMap<Id, BTreeSet<Id>> = BTreeMap::new();
    for (r, t) in relation_rows(log)? {
        result.entry(t).or_default().insert(r.activity);
    }
    Ok(result)
}
