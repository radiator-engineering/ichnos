//! Seeded random sampling of cases and events. Port of pm4py's
//! `sample_cases` and `sample_events`.
//!
//! pm4py draws with Python's `random` module, whose sequence ichnos cannot
//! reproduce. ichnos draws with ChaCha8 from an explicit seed, so a seed gives
//! the same sample on every platform. The sample keeps the input order;
//! pm4py's `random.sample` returns it in random order.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::log::{EventLog, EventStream};

/// Indices of `amount` of `len` items, drawn without replacement, ascending.
fn sample_indices(len: usize, amount: usize, seed: u64) -> Vec<usize> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut picked = rand::seq::index::sample(&mut rng, len, amount.min(len)).into_vec();
    picked.sort_unstable();
    picked
}

impl EventLog {
    /// A log with `n` traces drawn at random without replacement (all traces
    /// if the log has fewer), in log order. The log metadata is copied.
    pub fn sample_cases(&self, n: usize, seed: u64) -> EventLog {
        let traces = sample_indices(self.traces.len(), n, seed)
            .into_iter()
            .map(|i| self.traces[i].clone())
            .collect();
        EventLog {
            attributes: self.attributes.clone(),
            extensions: self.extensions.clone(),
            globals: self.globals.clone(),
            classifiers: self.classifiers.clone(),
            traces,
        }
    }
}

impl EventStream {
    /// A stream with `n` events drawn at random without replacement (all
    /// events if the stream has fewer), in stream order. The metadata is
    /// copied.
    pub fn sample_events(&self, n: usize, seed: u64) -> EventStream {
        let events = sample_indices(self.events.len(), n, seed)
            .into_iter()
            .map(|i| self.events[i].clone())
            .collect();
        EventStream {
            attributes: self.attributes.clone(),
            extensions: self.extensions.clone(),
            globals: self.globals.clone(),
            classifiers: self.classifiers.clone(),
            events,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::keys::EventKeys;
    use crate::log::EventLog;

    fn log() -> EventLog {
        let traces: Vec<String> = (0..50).map(|i| format!("A{i},B")).collect();
        EventLog::from_trace_strings(
            traces.iter().map(String::as_str),
            ",",
            &EventKeys::default(),
        )
    }

    fn ids(log: &EventLog) -> Vec<String> {
        log.iter()
            .map(|t| t.case_id().unwrap().to_string())
            .collect()
    }

    #[test]
    fn same_seed_same_sample_in_log_order() {
        let log = log();
        let a = log.sample_cases(10, 7);
        assert_eq!(a.len(), 10);
        assert_eq!(ids(&a), ids(&log.sample_cases(10, 7)));
        assert_ne!(ids(&a), ids(&log.sample_cases(10, 8)));
        let positions: Vec<usize> = ids(&a).iter().map(|id| id.parse().unwrap()).collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn oversized_sample_returns_everything() {
        let log = log();
        assert_eq!(log.sample_cases(500, 1), log);
        let stream = log.to_event_stream(&EventKeys::default());
        let sample = stream.sample_events(30, 3);
        assert_eq!(sample.len(), 30);
        assert_eq!(stream.sample_events(1000, 3), stream);
    }
}
