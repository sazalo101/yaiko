//! Bounded data export/import helpers for JSON and CSV payloads.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPayload {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataTransferError {
    Serialization,
    InvalidInput,
    TooLarge,
    UnsupportedFormat,
    InvalidFilename,
}

pub fn export_json<T: Serialize>(
    value: &T,
    filename: &str,
    max_bytes: usize,
) -> Result<ExportPayload, DataTransferError> {
    let bytes = serde_json::to_vec(value).map_err(|_| DataTransferError::Serialization)?;
    make_payload(bytes, "application/json", filename, max_bytes)
}

pub fn export_csv(
    rows: &[BTreeMap<String, String>],
    filename: &str,
    max_bytes: usize,
) -> Result<ExportPayload, DataTransferError> {
    let mut columns = Vec::new();
    for row in rows {
        for key in row.keys() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }
    let mut output = String::new();
    output.push_str(
        &columns
            .iter()
            .map(|column| csv_cell(column))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push('\n');
    for row in rows {
        output.push_str(
            &columns
                .iter()
                .map(|column| csv_cell(row.get(column).map(String::as_str).unwrap_or_default()))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    make_payload(
        output.into_bytes(),
        "text/csv; charset=utf-8",
        filename,
        max_bytes,
    )
}

pub fn import_json<T: DeserializeOwned>(
    bytes: &[u8],
    max_bytes: usize,
) -> Result<T, DataTransferError> {
    if bytes.len() > max_bytes {
        return Err(DataTransferError::TooLarge);
    }
    serde_json::from_slice(bytes).map_err(|_| DataTransferError::InvalidInput)
}

pub fn import_json_value(bytes: &[u8], max_bytes: usize) -> Result<Value, DataTransferError> {
    import_json(bytes, max_bytes)
}

fn make_payload(
    bytes: Vec<u8>,
    content_type: &str,
    filename: &str,
    max_bytes: usize,
) -> Result<ExportPayload, DataTransferError> {
    if bytes.len() > max_bytes {
        return Err(DataTransferError::TooLarge);
    }
    let filename = safe_filename(filename)?;
    Ok(ExportPayload {
        bytes,
        content_type: content_type.to_string(),
        filename,
    })
}

pub fn safe_filename(filename: &str) -> Result<String, DataTransferError> {
    let trimmed = filename.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.chars().any(|character| character.is_control())
    {
        return Err(DataTransferError::InvalidFilename);
    }
    Ok(trimmed.to_string())
}

fn csv_cell(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exports_json_with_metadata_and_round_trips_import() {
        let payload = export_json(&json!({"id": 1}), "records.json", 100).unwrap();
        assert_eq!(payload.content_type, "application/json");
        let value: Value = import_json(&payload.bytes, 100).unwrap();
        assert_eq!(value["id"], 1);
    }

    #[test]
    fn exports_csv_with_headers_and_escaping() {
        let rows = vec![BTreeMap::from([(
            String::from("name"),
            String::from("Ada, Lovelace"),
        )])];
        let payload = export_csv(&rows, "users.csv", 100).unwrap();
        assert_eq!(
            String::from_utf8(payload.bytes).unwrap(),
            "name\n\"Ada, Lovelace\"\n"
        );
    }

    #[test]
    fn enforces_size_and_filename_safety() {
        assert_eq!(
            export_json(&json!({"large": "payload"}), "safe.json", 1),
            Err(DataTransferError::TooLarge)
        );
        assert_eq!(
            safe_filename("../secret.json"),
            Err(DataTransferError::InvalidFilename)
        );
        assert_eq!(safe_filename("report.csv").unwrap(), "report.csv");
    }
}
