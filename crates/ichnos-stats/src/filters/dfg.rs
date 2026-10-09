use crate::{Error, Result};
use ichnos_model::dfg::{ActivityCounts, Dfg};

pub(super) fn fraction(value: f64) -> Result<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(Error::InvalidOption(
            "fraction must be finite and between zero and one",
        ));
    }
    Ok(())
}
fn frequencies(dfg: &Dfg) -> ActivityCounts {
    let mut incoming = ActivityCounts::new();
    let mut outgoing = ActivityCounts::new();
    for ((a, b), n) in &dfg.graph {
        *outgoing.entry(a.clone()).or_default() += n;
        *incoming.entry(b.clone()).or_default() += n;
    }
    dfg.edge_activities()
        .into_iter()
        .map(|a| {
            let n = incoming
                .get(&a)
                .copied()
                .unwrap_or(0)
                .max(outgoing.get(&a).copied().unwrap_or(0));
            (a, n)
        })
        .collect()
}
/// Filter DFG activities with the model implementation, using maximum incoming/outgoing totals.
/// pm4py's default percentage is 0.2.
pub fn filter_dfg_activities_percentage(dfg: &Dfg, percentage: f64) -> Result<Dfg> {
    fraction(percentage)?;
    Ok(dfg
        .filter_activities_percentage(&frequencies(dfg), percentage)
        .0)
}
/// Filter DFG paths with the model implementation, preserving retained activity connectivity.
/// pm4py's default percentage is 0.2; keep_all_activities is false.
pub fn filter_dfg_paths_percentage(dfg: &Dfg, percentage: f64) -> Result<Dfg> {
    fraction(percentage)?;
    Ok(dfg
        .filter_paths_percentage(&frequencies(dfg), percentage, false)
        .0)
}
