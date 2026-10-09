use ichnos_core::chrono::DateTime;
use ichnos_stats::time::datetime_timestamp;

#[test]
fn python_datetime_epoch_bits_and_submicrosecond_truncation() {
    // Values from Python datetime.timestamp(), including negative epochs and offsets.
    for (text, bits) in [
        ("2024-01-01T00:00:00.000001+00:00", 0x41d9648020000004),
        ("1969-12-31T23:59:59.999999+00:00", 0xbeb0c6f7a0b5ed8d),
        ("2500-01-01T00:00:00.123456+02:00", 0x420f2734a300fcd6),
        ("1900-01-01T00:00:00.123456-05:00", 0xc1e0754705fc0ca6),
        ("2024-01-01T00:00:00.000001999+00:00", 0x41d9648020000004),
        ("1969-12-31T23:59:59.999999999+00:00", 0xbeb0c6f7a0b5ed8d),
    ] {
        assert_eq!(
            datetime_timestamp(DateTime::parse_from_rfc3339(text).unwrap()).to_bits(),
            bits,
            "{text}"
        );
    }
}
