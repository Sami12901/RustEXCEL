use exel_workbook::Workbook;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("File save error: {0}")]
    SaveFailed(String),
    #[error("Unsupported or unrecognized file format for '{0}'")]
    UnsupportedFormat(String),
    #[error("Serialization / parsing error: {0}")]
    Serialization(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
}

/// Recognized spreadsheet file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileFormat {
    Xlsx,
    Xlsm,
    Xlsb,
    Csv,
    Tsv,
    Json,
    Unknown,
}

impl FileFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            FileFormat::Xlsx => "xlsx",
            FileFormat::Xlsm => "xlsm",
            FileFormat::Xlsb => "xlsb",
            FileFormat::Csv => "csv",
            FileFormat::Tsv => "tsv",
            FileFormat::Json => "json",
            FileFormat::Unknown => "",
        }
    }
}

/// Sniffs the file format from a path extension or filename.
pub fn detect_format<P: AsRef<Path>>(path: P) -> FileFormat {
    let path = path.as_ref();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        match ext.to_ascii_lowercase().as_str() {
            "xlsx" => FileFormat::Xlsx,
            "xlsm" => FileFormat::Xlsm,
            "xlsb" => FileFormat::Xlsb,
            "csv" => FileFormat::Csv,
            "tsv" | "tab" => FileFormat::Tsv,
            "json" => FileFormat::Json,
            _ => FileFormat::Unknown,
        }
    } else {
        FileFormat::Unknown
    }
}

/// Sniffs the file format from initial header bytes (ZIP PK signature for XLSX/XLSM, JSON brackets, etc.).
pub fn detect_format_from_bytes(bytes: &[u8]) -> FileFormat {
    if bytes.len() >= 4 && &bytes[0..4] == b"PK\x03\x04" {
        // Zip archive - likely XLSX or XLSM
        return FileFormat::Xlsx;
    }

    let trimmed = bytes
        .iter()
        .take(100)
        .filter(|b| !b.is_ascii_whitespace())
        .copied()
        .collect::<Vec<u8>>();

    if !trimmed.is_empty() && (trimmed[0] == b'{' || trimmed[0] == b'[') {
        return FileFormat::Json;
    }

    FileFormat::Unknown
}

/// Generic trait for spreadsheet file decoders.
pub trait SpreadsheetReader {
    fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Workbook, IoError>;
    fn read_bytes(&self, bytes: &[u8]) -> Result<Workbook, IoError>;
}

/// Generic trait for spreadsheet file encoders.
pub trait SpreadsheetWriter {
    fn write_file<P: AsRef<Path>>(&self, wb: &Workbook, path: P) -> Result<(), IoError>;
    fn write_bytes(&self, wb: &Workbook) -> Result<Vec<u8>, IoError>;
}

/// Atomically saves data to `target_path` using a temporary file in the same directory.
///
/// If `write_fn` encounters an error, the temporary file is dropped and deleted,
/// leaving any pre-existing file at `target_path` completely untouched and uncorrupted.
pub fn save_atomic<P: AsRef<Path>, F>(target_path: P, write_fn: F) -> Result<(), IoError>
where
    F: FnOnce(&mut File) -> Result<(), IoError>,
{
    let target = target_path.as_ref();
    let parent = target.parent().unwrap_or_else(|| Path::new("."));

    std::fs::create_dir_all(parent)?;

    // Create tempfile in the target directory to ensure same filesystem for atomic rename
    let mut temp = tempfile::Builder::new()
        .prefix(".exel_tmp_")
        .tempfile_in(parent)
        .map_err(|e| IoError::SaveFailed(format!("Failed to create temporary file: {e}")))?;

    // Execute writer closure
    write_fn(temp.as_file_mut())?;

    // Flush and sync to disk before committing rename
    temp.as_file().sync_all()?;

    // Atomically persist over target
    temp.persist(target).map_err(|e| {
        IoError::SaveFailed(format!(
            "Failed to atomically persist file to {}: {e}",
            target.display()
        ))
    })?;

    Ok(())
}

/// Convenient helper for atomic writing raw byte buffers.
pub fn save_atomic_bytes<P: AsRef<Path>>(target_path: P, data: &[u8]) -> Result<(), IoError> {
    save_atomic(target_path, |file| {
        file.write_all(data)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        assert_eq!(detect_format("budget.xlsx"), FileFormat::Xlsx);
        assert_eq!(detect_format("macro.xlsm"), FileFormat::Xlsm);
        assert_eq!(detect_format("data.csv"), FileFormat::Csv);
        assert_eq!(detect_format("export.tsv"), FileFormat::Tsv);
        assert_eq!(detect_format("records.json"), FileFormat::Json);
        assert_eq!(detect_format("unknown.xyz"), FileFormat::Unknown);

        assert_eq!(
            detect_format_from_bytes(b"PK\x03\x04\x14\x00"),
            FileFormat::Xlsx
        );
        assert_eq!(
            detect_format_from_bytes(b"   {\"rows\": []}"),
            FileFormat::Json
        );
        assert_eq!(detect_format_from_bytes(b"   [1, 2, 3]"), FileFormat::Json);
    }

    #[test]
    fn test_atomic_save_success_and_overwrite() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_save.txt");

        // First save
        save_atomic_bytes(&file_path, b"Initial Content").unwrap();
        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "Initial Content"
        );

        // Overwrite save
        save_atomic_bytes(&file_path, b"Updated Content").unwrap();
        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "Updated Content"
        );
    }

    #[test]
    fn test_atomic_save_failure_leaves_original_untouched() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("safe_file.txt");

        // Initial file
        save_atomic_bytes(&file_path, b"Safe Original Data").unwrap();

        // Attempt failing save
        let result = save_atomic(&file_path, |file| {
            file.write_all(b"Partial data that breaks").unwrap();
            Err(IoError::SaveFailed("Simulated write interruption".into()))
        });

        assert!(result.is_err());

        // Verify original file is still intact and completely uncorrupted!
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Safe Original Data");
    }
}
