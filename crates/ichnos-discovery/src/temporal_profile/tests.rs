use super::*;

#[test]
fn mean_stdev_matches_python_statistics() {
    assert_eq!(mean_stdev(&[5.0]), (5.0, 0.0));
    let (mean, stdev) = mean_stdev(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
    assert_eq!(mean, 5.0);
    // statistics.stdev([2, 4, 4, 4, 5, 5, 7, 9])
    assert!((stdev - 2.138_089_935_299_395).abs() < 1e-12);
}

#[test]
fn empty_log_has_empty_profile() {
    let profile = discover_temporal_profile(
        &EventLog::default(),
        &EventKeys::default(),
        &TemporalProfileOptions::default(),
    )
    .unwrap();
    assert!(profile.is_empty());
}
