//! DECLARE playout filters next events by prefix violations, as pm4py does.
//! It does not enforce outstanding existence/response obligations at trace end.
use crate::{PlayOutOptions, SimulationError};
use ichnos_core::{Event, EventLog, Trace};
use ichnos_discovery::declare::{DeclareActivities, DeclareModel, DeclareTemplate};
use rand::{Rng, RngExt};
use std::collections::BTreeSet;

/// Generates uniform choices among activities whose next event is prefix-safe.
/// A trace stops early when no activity is safe, including inconsistent models.
pub fn play_out_declare<R: Rng + ?Sized>(
    model: &DeclareModel,
    o: &PlayOutOptions,
    rng: &mut R,
) -> Result<EventLog, SimulationError> {
    if o.declare_min_length > o.declare_max_length || o.declare_max_length > o.max_steps {
        return Err(SimulationError::Options("invalid DECLARE length bounds"));
    }
    let mut activities = BTreeSet::new();
    for (template, rules) in &model.rules {
        for rule in rules.keys() {
            match rule {
                DeclareActivities::Unary(a) if template.is_unary() => {
                    activities.insert(a.as_str());
                }
                DeclareActivities::Binary(a, b) if !template.is_unary() => {
                    activities.insert(a.as_str());
                    activities.insert(b.as_str());
                }
                _ => return Err(SimulationError::Options("DECLARE rule arity mismatch")),
            }
        }
    }
    let mut log = EventLog::default();
    for i in 0..o.traces {
        let length = rng.random_range(o.declare_min_length..=o.declare_max_length);
        let mut prefix = Vec::new();
        for _ in 0..length {
            let candidates: Vec<_> = activities
                .iter()
                .copied()
                .filter(|a| safe(model, &prefix, a))
                .collect();
            if candidates.is_empty() {
                break;
            }
            prefix.push(candidates[rng.random_range(0..candidates.len())]);
        }
        let mut trace = Trace::with_case_id(i.to_string());
        for a in prefix {
            let mut event = Event::new();
            event.insert("concept:name", a);
            trace.events.push(event);
        }
        log.traces.push(trace);
    }
    Ok(log)
}
fn safe(model: &DeclareModel, prefix: &[&str], next: &str) -> bool {
    model.rules.iter().all(|(template, rules)| {
        rules.keys().all(|rule| {
            let (a, b) = match rule {
                DeclareActivities::Unary(a) => (a.as_str(), ""),
                DeclareActivities::Binary(a, b) => (a.as_str(), b.as_str()),
            };
            !violates(*template, a, b, prefix, next)
        })
    })
}
fn violates(t: DeclareTemplate, a: &str, b: &str, prefix: &[&str], next: &str) -> bool {
    use DeclareTemplate::*;
    let mut sequence = prefix.to_vec();
    sequence.push(next);
    match t {
        Existence | RespondedExistence | Coexistence | Response => false,
        Absence => sequence.contains(&a),
        ExactlyOne => sequence.iter().filter(|x| **x == a).count() > 1,
        Init => sequence.first() != Some(&a),
        Precedence | Succession => {
            let mut seen = false;
            for x in sequence {
                if x == b && !seen {
                    return true;
                }
                if x == a {
                    seen = true;
                }
            }
            false
        }
        AlternateResponse => {
            let mut waiting = false;
            for x in sequence {
                if x == a {
                    if waiting {
                        return true;
                    }
                    waiting = true;
                }
                if x == b {
                    waiting = false;
                }
            }
            false
        }
        AlternatePrecedence => {
            let mut waiting = true;
            for x in sequence {
                if x == b {
                    if waiting {
                        return true;
                    }
                    waiting = true;
                }
                if x == a {
                    waiting = false;
                }
            }
            false
        }
        AlternateSuccession => {
            let mut waiting_b = false;
            let mut waiting_a = true;
            for x in sequence {
                if x == a {
                    if waiting_b {
                        return true;
                    }
                    waiting_b = true;
                    waiting_a = false;
                } else if x == b {
                    if waiting_a {
                        return true;
                    }
                    waiting_a = true;
                    waiting_b = false;
                }
            }
            false
        }
        ChainResponse => sequence.windows(2).any(|w| w[0] == a && w[1] != b),
        ChainPrecedence => sequence
            .iter()
            .enumerate()
            .any(|(i, x)| *x == b && (i == 0 || sequence[i - 1] != a)),
        ChainSuccession => {
            sequence.windows(2).any(|w| w[0] == a && w[1] != b)
                || sequence
                    .iter()
                    .enumerate()
                    .any(|(i, x)| *x == b && (i == 0 || sequence[i - 1] != a))
        }
        NonCoexistence => sequence.contains(&a) && sequence.contains(&b),
        NonSuccession => {
            let mut seen = false;
            for x in sequence {
                if x == a {
                    seen = true;
                }
                if x == b && seen {
                    return true;
                }
            }
            false
        }
        NonChainSuccession => sequence.windows(2).any(|w| w[0] == a && w[1] == b),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use ichnos_discovery::declare::DeclareCounts;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use serde_json::json;
    #[test]
    fn all_prefix_automata_match_golden() {
        let g = ichnos_golden::golden("simulation", "playout-declare-prefixes");
        for item in g.expected.as_array().unwrap() {
            let template = DeclareTemplate::ALL
                .into_iter()
                .find(|t| Some(t.as_str()) == item["template"].as_str())
                .unwrap();
            let a = item["a"].as_str().unwrap();
            let b = item["b"].as_str().unwrap();
            for row in item["rows"].as_array().unwrap() {
                let prefix: Vec<_> = row["prefix"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect();
                let allowed: Vec<_> = ["a", "b", "c"]
                    .into_iter()
                    .filter(|next| !violates(template, a, b, &prefix, next))
                    .collect();
                assert_eq!(
                    json!(allowed),
                    row["allowed"],
                    "{} {row}",
                    template.as_str()
                );
            }
        }
    }
    #[test]
    fn dispatcher_observes_init_precedence_and_length() {
        let mut model = DeclareModel::default();
        model
            .rules
            .entry(DeclareTemplate::Init)
            .or_default()
            .insert(
                DeclareActivities::Unary("a".into()),
                DeclareCounts::default(),
            );
        model
            .rules
            .entry(DeclareTemplate::Precedence)
            .or_default()
            .insert(
                DeclareActivities::Binary("a".into(), "b".into()),
                DeclareCounts::default(),
            );
        let o = PlayOutOptions {
            traces: 100,
            ..Default::default()
        };
        let log = crate::play_out(
            crate::Model::Declare(&model),
            &o,
            &mut ChaCha8Rng::seed_from_u64(1729),
        )
        .unwrap();
        assert_eq!(log.traces.len(), 100);
        assert!(log.traces.iter().all(|t| (3..=15).contains(&t.len())
            && t.events[0].get("concept:name").unwrap().as_str() == Some("a")));
    }
}
