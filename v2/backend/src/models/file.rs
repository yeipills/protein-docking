/*!
 * File models - Gestión de archivos
 */

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: Uuid,
    pub original_name: String,
    pub file_type: FileType,
    pub size_bytes: u64,
    pub checksum: String,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    pub processed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: FileMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedFile {
    pub file_info: FileInfo,
    pub processing_status: ProcessingStatus,
    pub validation_results: Option<ValidationResults>,
    pub parsed_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    PDB,
    STL,
    Vertices,
    Faces,
    Results,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub content_type: String,
    pub encoding: Option<String>,
    pub structure_info: Option<StructureInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureInfo {
    pub atom_count: Option<u32>,
    pub residue_count: Option<u32>,
    pub chain_count: Option<u8>,
    pub resolution: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub statistics: ValidationStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub line_number: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub error_type: ValidationErrorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub line_number: Option<u32>,
    pub message: String,
    pub warning_type: ValidationWarningType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatistics {
    pub total_lines: u32,
    pub valid_records: u32,
    pub invalid_records: u32,
    pub duplicate_records: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    InvalidFormat,
    MissingRequiredField,
    InvalidValue,
    StructuralError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationWarningType {
    UnusualValue,
    MissingOptionalField,
    FormatInconsistency,
}

impl FileInfo {
    pub fn new(original_name: String, file_type: FileType, size_bytes: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            original_name,
            file_type,
            size_bytes,
            checksum: String::new(), // Will be calculated
            uploaded_at: chrono::Utc::now(),
            processed_at: None,
            metadata: FileMetadata {
                content_type: "application/octet-stream".to_string(),
                encoding: None,
                structure_info: None,
            },
        }
    }
    
    pub fn is_valid_size(&self, max_size: u64) -> bool {
        self.size_bytes <= max_size
    }
    
    pub fn age(&self) -> chrono::Duration {
        chrono::Utc::now() - self.uploaded_at
    }
}