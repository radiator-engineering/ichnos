use crate::{Error, Result};
use ichnos_core::{EventKeys, EventLog};
use std::collections::{BTreeMap, BTreeSet};

/// Information-theoretic metrics for one activity.
#[derive(Clone, Debug)]
pub struct ChaoticActivity {
    /// Activity name.
    pub activity: String,
    /// Event frequency.
    pub freq: usize,
    /// Sum of predecessor and successor entropy.
    pub entropy: f64,
    /// Laplace-smoothed entropy.
    pub entropy_smooth: f64,
    /// Reduction in total entropy after removing this activity.
    pub entropy_gain: f64,
    /// Mean of smoothed entropy and entropy gain.
    pub chaotic_score: f64,
}
fn entropies(traces: &[Vec<String>], alpha: Option<f64>) -> BTreeMap<String, (usize, f64)> {
    let mut counts = BTreeMap::<String, usize>::new();
    let mut pairs = BTreeMap::<(String, String), usize>::new();
    for t in traces {
        let mut previous = "▶";
        for a in t {
            *counts.entry(a.clone()).or_default() += 1;
            *pairs.entry((previous.to_owned(), a.clone())).or_default() += 1;
            previous = a;
        }
        *pairs
            .entry((previous.to_owned(), "■".to_owned()))
            .or_default() += 1;
    }
    counts
        .iter()
        .map(|(a, &n)| {
            let denom = n as f64 + alpha.unwrap_or(0.0) * (counts.len() + 1) as f64;
            let mut entropy = 0.0;
            for follows in [false, true] {
                let mut others = counts.keys().cloned().collect::<BTreeSet<_>>();
                others.insert(if follows { "■" } else { "▶" }.to_owned());
                for b in others {
                    let pair = if follows {
                        (a.clone(), b)
                    } else {
                        (b, a.clone())
                    };
                    let p = (*pairs.get(&pair).unwrap_or(&0) as f64 + alpha.unwrap_or(0.0)) / denom;
                    if p > 0.0 {
                        entropy -= p * p.log2();
                    }
                }
            }
            (a.clone(), (n, entropy))
        })
        .collect()
}
/// Total directly-follows and directly-precedes entropy, optionally smoothed.
pub fn total_entropy(traces: &[Vec<String>], alpha: Option<f64>) -> Result<f64> {
    if alpha.is_some_and(|a| !a.is_finite() || a < 0.0) {
        return Err(Error::InvalidOption("alpha must be nonnegative and finite"));
    }
    Ok(entropies(traces, alpha).values().map(|v| v.1).sum())
}
/// Chaotic metrics, defaulting to smoothing alpha = 1 / number of activities.
pub fn chaotic_metrics(traces: &[Vec<String>], alpha: Option<f64>) -> Result<Vec<ChaoticActivity>> {
    let baseline = total_entropy(traces, None)?;
    let raw = entropies(traces, None);
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let alpha = alpha.unwrap_or(1.0 / raw.len() as f64);
    total_entropy(traces, Some(alpha))?;
    let smooth = entropies(traces, Some(alpha));
    let mut result = Vec::new();
    for (a, (freq, entropy)) in raw {
        let filtered: Vec<Vec<_>> = traces
            .iter()
            .map(|t| t.iter().filter(|v| **v != a).cloned().collect::<Vec<_>>())
            .filter(|t| !t.is_empty())
            .collect();
        let entropy_gain = baseline - total_entropy(&filtered, None)?;
        let entropy_smooth = smooth[&a].1;
        result.push(ChaoticActivity {
            activity: a,
            freq,
            entropy,
            entropy_smooth,
            entropy_gain,
            chaotic_score: (entropy_smooth + entropy_gain) / 2.0,
        });
    }
    result.sort_by(|a, b| {
        b.chaotic_score
            .total_cmp(&a.chaotic_score)
            .then_with(|| b.freq.cmp(&a.freq))
            .then_with(|| a.activity.cmp(&b.activity))
    });
    Ok(result)
}
/// Chaotic activity detection on an event log.
pub fn get_chaotic_activities(
    log: &EventLog,
    keys: &EventKeys,
    alpha: Option<f64>,
) -> Result<Vec<ChaoticActivity>> {
    let seq = log.activity_sequences(keys)?;
    chaotic_metrics(
        &seq.traces
            .iter()
            .map(|t| {
                t.iter()
                    .map(|&a| seq.activities.name(a).to_owned())
                    .collect()
            })
            .collect::<Vec<_>>(),
        alpha,
    )
}
