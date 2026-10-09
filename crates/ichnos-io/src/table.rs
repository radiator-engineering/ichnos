use std::sync::Arc;

use ichnos_core::{
    EventKeys, EventLog,
    arrow::{
        array::{
            Array, ArrayRef, AsArray, BooleanArray, Float32Array, Float64Array, StringArray,
            TimestampNanosecondArray,
        },
        compute::{and, cast, concat_batches, filter_record_batch, is_not_null},
        datatypes::{DataType, Field, Schema},
        record_batch::RecordBatch,
    },
    chrono::{DateTime, NaiveDate, NaiveDateTime},
};

use crate::Result;

fn timestamp(text: &str, format: Option<&str>) -> Option<i64> {
    let text = text.trim();
    if format.is_none()
        && let Ok(date) = DateTime::parse_from_rfc3339(text)
    {
        return date.timestamp_nanos_opt();
    }
    let formats = format.map_or_else(
        || {
            vec![
                "%Y-%m-%d %H:%M:%S%.f%:z",
                "%Y-%m-%d %H:%M:%S%.f%z",
                "%Y-%m-%dT%H:%M:%S%.f%:z",
                "%Y-%m-%dT%H:%M:%S%.f%z",
                "%Y-%m-%d %H:%M:%S%.f",
                "%Y-%m-%dT%H:%M:%S%.f",
                "%Y-%m-%d %H:%M",
                "%Y-%m-%dT%H:%M",
                "%Y/%m/%d %H:%M:%S%.f",
                "%Y/%m/%d %H:%M",
                "%Y-%m-%d",
                "%Y/%m/%d",
            ]
        },
        |format| vec![format],
    );
    for format in formats {
        if let Ok(date) = DateTime::parse_from_str(text, format) {
            return date.timestamp_nanos_opt();
        }
        if let Ok(date) = NaiveDateTime::parse_from_str(text, format) {
            return date.and_utc().timestamp_nanos_opt();
        }
        if let Ok(date) = NaiveDate::parse_from_str(text, format) {
            return date.and_hms_opt(0, 0, 0)?.and_utc().timestamp_nanos_opt();
        }
    }
    None
}

/// Attempts whole-column datetime conversion, as pandas does. A mixed column
/// stays textual; null values do not prevent conversion.
fn datetime_column(column: &ArrayRef, format: Option<&str>) -> Option<ArrayRef> {
    if !matches!(
        column.data_type(),
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View | DataType::Dictionary(..)
    ) {
        return None;
    }
    let column = cast(column, &DataType::Utf8).ok()?;
    let strings: &StringArray = column.as_string::<i32>();
    if strings.null_count() == strings.len() {
        return None;
    }
    let values = strings
        .iter()
        .map(|v| {
            v.map(|s| timestamp(s, format))
                .map_or(Some(None), |v| v.map(Some))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Arc::new(
        TimestampNanosecondArray::from(values).with_timezone("UTC"),
    ))
}

pub(crate) fn to_log(
    schema: Arc<Schema>,
    batches: &[RecordBatch],
    keys: &EventKeys,
    format: Option<&str>,
) -> Result<EventLog> {
    // Sorting each batch separately would split cases and reorder equal timestamps.
    let mut batch = concat_batches(&schema, batches)?;
    if batch.num_rows() == 0 && schema.fields().is_empty() {
        return Ok(EventLog::default());
    }
    // pandas treats numeric NaN as missing, even when Arrow has no null bit.
    let columns: Vec<ArrayRef> = batch
        .columns()
        .iter()
        .map(|column| match column.data_type() {
            DataType::Float64 => Arc::new(Float64Array::from(
                column
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .expect("float64 column")
                    .iter()
                    .map(|value| value.filter(|v| !v.is_nan()))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
            DataType::Float32 => Arc::new(Float32Array::from(
                column
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .expect("float32 column")
                    .iter()
                    .map(|value| value.filter(|v| !v.is_nan()))
                    .collect::<Vec<_>>(),
            )) as ArrayRef,
            _ => datetime_column(column, format).unwrap_or_else(|| column.clone()),
        })
        .collect();
    let fields = schema
        .fields()
        .iter()
        .zip(&columns)
        .map(|(field, column)| {
            field
                .as_ref()
                .clone()
                .with_data_type(column.data_type().clone())
        })
        .collect::<Vec<Field>>();
    batch = RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(fields, schema.metadata().clone())),
        columns,
    )?;
    let mut valid = BooleanArray::from(vec![true; batch.num_rows()]);
    for key in [&keys.case_id, &keys.activity, &keys.timestamp] {
        let index = schema
            .index_of(key)
            .map_err(|_| ichnos_core::Error::MissingColumn(key.clone()))?;
        valid = and(&valid, &is_not_null(batch.column(index))?)?;
    }
    batch = filter_record_batch(&batch, &valid)?;
    if batch.num_rows() == 0 {
        return Ok(EventLog::default());
    }
    let formatted = ichnos_core::format_batch(&batch, keys, format)?;
    Ok(EventLog::from_arrow(&formatted, &EventKeys::default())?)
}
