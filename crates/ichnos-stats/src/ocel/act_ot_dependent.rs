//! Activity associations within each object type.
use super::*;

/// Group relations by type, then deduplicate pairs and select endpoints within each type.
pub fn find_associations_from_ocel(
    log: &Ocel,
    prefilter: Prefilter,
) -> Result<TypedActivityAssociations> {
    let mut grouped: BTreeMap<Id, Vec<ActivityRelation>> = BTreeMap::new();
    for (r, t) in relation_rows(log)? {
        grouped.entry(t).or_default().push(r);
    }
    Ok(grouped
        .into_iter()
        .map(|(t, rows)| {
            (
                t,
                act_utils::find_associations_from_relations_df(&rows, prefilter),
            )
        })
        .collect())
}
/// Distinct events per object type and activity.
pub fn aggregate_events(associations: &TypedActivityAssociations) -> TypedActivityIdentifiers {
    associations
        .iter()
        .map(|(t, a)| (t.clone(), act_utils::aggregate_events(a)))
        .collect()
}
/// Distinct objects per object type and activity.
pub fn aggregate_unique_objects(
    associations: &TypedActivityAssociations,
) -> TypedActivityIdentifiers {
    associations
        .iter()
        .map(|(t, a)| (t.clone(), act_utils::aggregate_unique_objects(a)))
        .collect()
}
/// Borrow the original occurrences unchanged, retaining duplicates and order.
pub fn aggregate_total_objects(
    associations: &TypedActivityAssociations,
) -> &TypedActivityAssociations {
    associations
}
