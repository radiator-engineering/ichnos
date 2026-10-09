//! Activity associations independent of object type.
use super::*;

/// Group distinct event/object pairs by activity, optionally selecting endpoints.
/// The first row wins when a pair occurs more than once.
pub fn find_associations_from_relations_df(
    rows: &[ActivityRelation],
    prefilter: Prefilter,
) -> ActivityAssociations {
    let mut seen = BTreeSet::new();
    let unique: Vec<_> = rows
        .iter()
        .filter(|r| seen.insert((&r.event, &r.object)))
        .collect();
    let selected = if prefilter == Prefilter::None {
        unique
    } else {
        let mut endpoints = BTreeMap::new();
        for r in unique {
            if prefilter == Prefilter::End {
                endpoints.insert(&r.object, r);
            } else {
                endpoints.entry(&r.object).or_insert(r);
            }
        }
        endpoints.into_values().collect()
    };
    let mut result: ActivityAssociations = BTreeMap::new();
    for r in selected {
        result
            .entry(r.activity.clone())
            .or_default()
            .push((r.event.clone(), r.object.clone()));
    }
    result
}

/// Associate activities with distinct event/object pairs using canonical OCEL fields.
/// Missing referenced events or objects return a typed error.
pub fn find_associations_from_ocel(
    log: &Ocel,
    prefilter: Prefilter,
) -> Result<ActivityAssociations> {
    let rows = relation_rows(log)?
        .into_iter()
        .map(|(r, _)| r)
        .collect::<Vec<_>>();
    Ok(find_associations_from_relations_df(&rows, prefilter))
}

/// Distinct event identifiers for each activity, including empty activity entries.
pub fn aggregate_events(associations: &ActivityAssociations) -> ActivityIdentifiers {
    project(associations, |pair| &pair.0)
}
/// Distinct object identifiers for each activity, including empty activity entries.
pub fn aggregate_unique_objects(associations: &ActivityAssociations) -> ActivityIdentifiers {
    project(associations, |pair| &pair.1)
}
fn project(
    associations: &ActivityAssociations,
    identifier: fn(&Association) -> &Id,
) -> ActivityIdentifiers {
    associations
        .iter()
        .map(|(a, pairs)| {
            (
                a.clone(),
                pairs.iter().map(|pair| identifier(pair).clone()).collect(),
            )
        })
        .collect()
}
/// Borrow the original occurrences unchanged, retaining duplicates and order.
/// pm4py returns the same input object for this aggregation.
pub fn aggregate_total_objects(associations: &ActivityAssociations) -> &ActivityAssociations {
    associations
}
