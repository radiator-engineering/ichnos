//! Train/test splits and prefixes, ported from pm4py's
//! `objects/log/util/split_train_test.py` and `get_prefixes.py`.

use ichnos_core::{EventLog, Trace};

/// Splits a log into a training log and a test log, as pm4py's
/// `split_train_test` does for an `EventLog`.
///
/// The trace positions are shuffled with Python's `random.shuffle`
/// algorithm: for `i` from `n - 1` down to 1, swap position `i` with
/// `rand_below(i + 1)`. `rand_below(k)` must return a whole number in
/// `0..k`; Python's `random._randbelow` gives the same split as pm4py. The
/// first `floor(n * train_percentage) + 1` shuffled traces form the
/// training log and the rest the test log. Both keep the log's attributes,
/// extensions, globals and classifiers.
///
/// pm4py's data-frame variant instead draws `random.random()` once per case
/// and keeps the case for training when the draw is at most
/// `train_percentage`; it is not ported.
pub fn split_train_test(
    log: &EventLog,
    train_percentage: f64,
    mut rand_below: impl FnMut(usize) -> usize,
) -> (EventLog, EventLog) {
    let n = log.traces.len();
    let mut idxs: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = rand_below(i + 1);
        idxs.swap(i, j);
    }
    let stop = ((n as f64 * train_percentage).floor() as usize + 1).min(n);
    let (train, test) = idxs.split_at(stop);
    (with_traces(log, train), with_traces(log, test))
}

/// Keeps at most the first `length` events of each trace, as pm4py's
/// `get_prefixes_from_log` does. Trace attributes and log metadata are kept.
pub fn get_prefixes_from_log(log: &EventLog, length: usize) -> EventLog {
    let mut out = empty_like(log);
    out.traces = log
        .traces
        .iter()
        .map(|t| Trace {
            attributes: t.attributes.clone(),
            events: t.events.iter().take(length).cloned().collect(),
        })
        .collect();
    out
}

fn empty_like(log: &EventLog) -> EventLog {
    EventLog {
        attributes: log.attributes.clone(),
        extensions: log.extensions.clone(),
        globals: log.globals.clone(),
        classifiers: log.classifiers.clone(),
        traces: Vec::new(),
    }
}

fn with_traces(log: &EventLog, idxs: &[usize]) -> EventLog {
    let mut out = empty_like(log);
    out.traces = idxs.iter().map(|&i| log.traces[i].clone()).collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(n: usize) -> EventLog {
        let mut log = EventLog::default();
        for i in 0..n {
            log.traces.push(Trace::with_case_id(i.to_string()));
        }
        log
    }

    fn ids(log: &EventLog) -> Vec<String> {
        log.traces
            .iter()
            .map(|t| t.case_id().unwrap().to_string())
            .collect()
    }

    #[test]
    fn identity_draws_keep_the_order_and_add_one_to_training() {
        let (train, test) = split_train_test(&log(5), 0.5, |k| k - 1);
        assert_eq!(ids(&train), ["0", "1", "2"]);
        assert_eq!(ids(&test), ["3", "4"]);
    }

    #[test]
    fn a_full_split_keeps_every_trace_for_training() {
        let (train, test) = split_train_test(&log(3), 1.0, |_| 0);
        assert_eq!(train.traces.len(), 3);
        assert!(test.traces.is_empty());
        let (train, test) = split_train_test(&log(0), 0.8, |_| 0);
        assert!(train.traces.is_empty() && test.traces.is_empty());
    }
}
