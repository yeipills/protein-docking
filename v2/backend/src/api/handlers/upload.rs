// Upload Handler - Manejo ultra-optimizado de upload de proteínas
// Soporta PDB, MOL, SDF con validación y streaming

use actix_web::{web, HttpResponse, Result, HttpRequest};
use actix_multipart::Multipart;
use serde::{Serialize, Deserialize};
use futures::{StreamExt, TryStreamExt};
use std::io::Write;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::algorithms::{ProteinParser, ProteinStructure};

#[cfg(feature = "postgres")]
use crate::database::{get_database_pool, ProteinFileModel};
#[cfg(not(feature = "postgres"))]
use crate::database::{get_simple_pool, SimpleProteinFileModel};

/// Respuesta de upload exitoso
#[derive(Serialize)]
pub struct UploadResponse {
    pub file_id: String,
    pub filename: String,
    pub file_size: u64,
    pub protein_info: ProteinInfo,
    pub upload_time_ms: u64,
    pub status: String,
}

/// Información básica de la proteína
#[derive(Serialize)]
pub struct ProteinInfo {
    pub name: String,
    pub pdb_id: Option<String>,
    pub total_atoms: usize,
    pub total_residues: usize,
    pub chains: usize,
    pub molecular_weight: f64,
    pub center_of_mass: [f64; 3],
    pub bounding_box_size: [f64; 3],
}

/// Configuración de upload
#[derive(Deserialize)]
pub struct UploadConfig {
    pub max_file_size: Option<u64>,
    pub allowed_formats: Option<Vec<String>>,
    pub validate_structure: Option<bool>,
}

/// Metadatos de archivo
#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_id: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: u64,
    pub content_type: String,
    pub upload_timestamp: chrono::DateTime<chrono::Utc>,
    pub protein_structure: Option<ProteinStructure>,
}

/// Upload de archivo de proteína con streaming
pub async fn upload_protein_file(
    mut payload: Multipart,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();
    info!("📤 Starting protein file upload");
    
    const MAX_FILE_SIZE: usize = 500 * 1024 * 1024; // 500MB
    const UPLOAD_DIR: &str = "./uploads";
    
    // Crear directorio de uploads si no existe
    std::fs::create_dir_all(UPLOAD_DIR).map_err(|e| {
        error!("Failed to create upload directory: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to create upload directory")
    })?;
    
    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition();
        
        if let Some(filename) = content_disposition.get_filename() {
            info!("📁 Processing file: {}", filename);
            
            // Validar formato de archivo
            if !is_supported_format(filename) {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Unsupported file format",
                    "supported_formats": ["pdb", "mol", "sdf"],
                    "filename": filename
                })));
            }
            
            // Generar ID único para el archivo
            let file_id = Uuid::new_v4().to_string();
            let file_extension = get_file_extension(filename);
            let stored_filename = format!("{}.{}", file_id, file_extension);
            let file_path = format!("{}/{}", UPLOAD_DIR, stored_filename);
            
            // Crear archivo temporal
            let mut file = web::block(move || std::fs::File::create(&file_path))
                .await
                .map_err(|e| {
                    error!("Failed to create file: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to create file")
                })??;
            
            let mut file_size = 0u64;
            let mut file_content = Vec::new();
            
            // Leer archivo en chunks con validación de tamaño
            while let Some(chunk) = field.try_next().await? {
                file_size += chunk.len() as u64;
                
                if file_size > MAX_FILE_SIZE as u64 {
                    // Eliminar archivo parcial
                    let _ = std::fs::remove_file(&file_path);
                    return Ok(HttpResponse::PayloadTooLarge().json(serde_json::json!({
                        "error": "File too large",
                        "max_size_mb": MAX_FILE_SIZE / (1024 * 1024),
                        "current_size_mb": file_size / (1024 * 1024)
                    })));
                }
                
                // Escribir chunk al archivo
                file = web::block(move || {
                    file.write_all(&chunk)?;
                    Ok::<_, std::io::Error>(file)
                })
                .await
                .map_err(|e| {
                    error!("Failed to write file chunk: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to write file")
                })??;
                
                // Guardar contenido para parsing
                file_content.extend_from_slice(&chunk);
            }
            
            // Finalizar escritura
            file = web::block(move || {
                file.flush()?;
                Ok::<_, std::io::Error>(file)
            })
            .await
            .map_err(|e| {
                error!("Failed to flush file: {}", e);
                actix_web::error::ErrorInternalServerError("Failed to finalize file")
            })??;
            
            info!("✅ File saved: {} ({} bytes)", filename, file_size);
            
            // Parsear estructura de proteína
            let protein_structure = match parse_protein_file(&file_content, filename, &file_extension).await {
                Ok(structure) => Some(structure),
                Err(e) => {
                    warn!("Failed to parse protein structure: {}", e);
                    // No eliminar archivo, pero marcar como no parseado
                    None
                }
            };
            
            // Crear metadatos
            let metadata = FileMetadata {
                file_id: file_id.clone(),
                original_filename: filename.to_string(),
                file_path: file_path.clone(),
                file_size,
                content_type: field.content_type().to_string(),
                upload_timestamp: chrono::Utc::now(),
                protein_structure: protein_structure.clone(),
            };
            
            // Guardar metadatos (en producción iría a base de datos)
            save_file_metadata(&metadata).await?;
            
            let upload_time = start_time.elapsed();
            
            // Crear respuesta
            let response = if let Some(structure) = protein_structure {
                UploadResponse {
                    file_id,
                    filename: filename.to_string(),
                    file_size,
                    protein_info: ProteinInfo {
                        name: structure.name.clone(),
                        pdb_id: structure.pdb_id.clone(),
                        total_atoms: structure.atoms.len(),
                        total_residues: structure.residues.len(),
                        chains: structure.chains.len(),
                        molecular_weight: structure.metadata.molecular_weight,
                        center_of_mass: structure.center_of_mass,
                        bounding_box_size: structure.bbox.size,
                    },
                    upload_time_ms: upload_time.as_millis() as u64,
                    status: "parsed".to_string(),
                }
            } else {
                UploadResponse {
                    file_id,
                    filename: filename.to_string(),
                    file_size,
                    protein_info: ProteinInfo {
                        name: "Unknown".to_string(),
                        pdb_id: None,
                        total_atoms: 0,
                        total_residues: 0,
                        chains: 0,
                        molecular_weight: 0.0,
                        center_of_mass: [0.0; 3],
                        bounding_box_size: [0.0; 3],
                    },
                    upload_time_ms: upload_time.as_millis() as u64,
                    status: "uploaded_not_parsed".to_string(),
                }
            };
            
            info!("🎉 Upload completed: {} in {:?}", filename, upload_time);
            
            return Ok(HttpResponse::Ok().json(response));
        }
    }
    
    Ok(HttpResponse::BadRequest().json(serde_json::json!({
        "error": "No file found in request"
    })))
}

/// Download de archivo de proteína
pub async fn download_protein_file(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let file_id = path.into_inner();
    info!("📥 Download request for file: {}", file_id);
    
    // Buscar archivo en metadatos
    match get_file_metadata(&file_id).await {
        Ok(Some(metadata)) => {
            // Verificar que el archivo existe
            if !std::path::Path::new(&metadata.file_path).exists() {
                return Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": "File not found on disk",
                    "file_id": file_id
                })));
            }
            
            // Leer archivo
            let file_content = web::block(move || {
                std::fs::read(&metadata.file_path)
            })
            .await
            .map_err(|e| {
                error!("Failed to read file: {}", e);
                actix_web::error::ErrorInternalServerError("Failed to read file")
            })??;
            
            info!("✅ File downloaded: {} ({} bytes)", metadata.original_filename, file_content.len());
            
            Ok(HttpResponse::Ok()
                .content_type(&metadata.content_type)
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", metadata.original_filename)))
                .insert_header(("X-File-ID", metadata.file_id))
                .insert_header(("X-Upload-Date", metadata.upload_timestamp.to_rfc3339()))
                .body(file_content))
        },
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "File not found",
                "file_id": file_id
            })))
        },
        Err(e) => {
            error!("Database error: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database error"
            })))
        }
    }
}

/// Obtener información de archivo sin descargarlo
pub async fn get_file_info(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let file_id = path.into_inner();
    info!("ℹ️ File info request for: {}", file_id);
    
    match get_file_metadata(&file_id).await {
        Ok(Some(metadata)) => {
            let response = serde_json::json!({
                "file_id": metadata.file_id,
                "filename": metadata.original_filename,
                "file_size": metadata.file_size,
                "content_type": metadata.content_type,
                "upload_timestamp": metadata.upload_timestamp,
                "protein_info": if let Some(structure) = &metadata.protein_structure {
                    serde_json::json!({
                        "name": structure.name,
                        "pdb_id": structure.pdb_id,
                        "total_atoms": structure.atoms.len(),
                        "total_residues": structure.residues.len(),
                        "chains": structure.chains.len(),
                        "molecular_weight": structure.metadata.molecular_weight,
                        "center_of_mass": structure.center_of_mass,
                        "bounding_box": structure.bbox
                    })
                } else {
                    serde_json::Value::Null
                },
                "status": if metadata.protein_structure.is_some() { "parsed" } else { "uploaded_only" }
            });
            
            Ok(HttpResponse::Ok().json(response))
        },
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "File not found",
                "file_id": file_id
            })))
        },
        Err(e) => {
            error!("Database error: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database error"
            })))
        }
    }
}

/// Listar archivos subidos
pub async fn list_uploaded_files(
    query: web::Query<ListFilesQuery>,
) -> Result<HttpResponse> {
    info!("📋 Listing uploaded files");
    
    let files = get_all_file_metadata(query.limit.unwrap_or(50), query.offset.unwrap_or(0)).await
        .map_err(|e| {
            error!("Failed to list files: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to list files")
        })?;
    
    let response = serde_json::json!({
        "files": files.iter().map(|metadata| {
            serde_json::json!({
                "file_id": metadata.file_id,
                "filename": metadata.original_filename,
                "file_size": metadata.file_size,
                "upload_timestamp": metadata.upload_timestamp,
                "has_protein_structure": metadata.protein_structure.is_some(),
                "status": if metadata.protein_structure.is_some() { "parsed" } else { "uploaded_only" }
            })
        }).collect::<Vec<_>>(),
        "total": files.len(),
        "limit": query.limit.unwrap_or(50),
        "offset": query.offset.unwrap_or(0)
    });
    
    Ok(HttpResponse::Ok().json(response))
}

#[derive(Deserialize)]
pub struct ListFilesQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Eliminar archivo
pub async fn delete_file(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let file_id = path.into_inner();
    info!("🗑️ Delete request for file: {}", file_id);
    
    match get_file_metadata(&file_id).await {
        Ok(Some(metadata)) => {
            // Eliminar archivo físico
            if std::path::Path::new(&metadata.file_path).exists() {
                web::block(move || {
                    std::fs::remove_file(&metadata.file_path)
                })
                .await
                .map_err(|e| {
                    error!("Failed to delete file: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to delete file")
                })??;
            }
            
            // Eliminar metadatos
            delete_file_metadata(&file_id).await
                .map_err(|e| {
                    error!("Failed to delete metadata: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to delete metadata")
                })?;
            
            info!("✅ File deleted: {}", metadata.original_filename);
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "File deleted successfully",
                "file_id": file_id,
                "filename": metadata.original_filename
            })))
        },
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "File not found",
                "file_id": file_id
            })))
        },
        Err(e) => {
            error!("Database error: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database error"
            })))
        }
    }
}

// Funciones auxiliares

fn is_supported_format(filename: &str) -> bool {
    let supported_extensions = ["pdb", "mol", "sdf", "mol2"];
    let extension = get_file_extension(filename);
    supported_extensions.contains(&extension.as_str())
}

fn get_file_extension(filename: &str) -> String {
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown")
        .to_lowercase()
}

async fn parse_protein_file(
    content: &[u8], 
    filename: &str, 
    extension: &str
) -> anyhow::Result<ProteinStructure> {
    let content_str = String::from_utf8(content.to_vec())?;
    let parser = ProteinParser::new();
    
    let structure = match extension {
        "pdb" => {
            let name = std::path::Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            parser.parse_pdb(&content_str, name)
        },
        _ => {
            anyhow::bail!("Unsupported format for parsing: {}", extension)
        }
    }?;
    
    Ok(structure)
}

// Funciones de persistencia con base de datos PostgreSQL o simple storage

async fn save_file_metadata(metadata: &FileMetadata) -> anyhow::Result<()> {
    #[cfg(feature = "postgres")]
    {
        let pool = get_database_pool()?;
        
        // Guardar en PostgreSQL
        let _model = ProteinFileModel::create(
            pool,
            metadata.file_id.clone(),
            metadata.original_filename.clone(),
            metadata.file_path.clone(),
            metadata.file_size as i64,
            metadata.content_type.clone(),
            metadata.protein_structure.clone(),
        ).await?;
    }
    
    #[cfg(not(feature = "postgres"))]
    {
        let pool = get_simple_pool()?;
        
        // Crear modelo simple
        let simple_model = SimpleProteinFileModel {
            id: uuid::Uuid::new_v4(),
            file_id: metadata.file_id.clone(),
            original_filename: metadata.original_filename.clone(),
            file_path: metadata.file_path.clone(),
            file_size: metadata.file_size as i64,
            content_type: metadata.content_type.clone(),
            upload_timestamp: metadata.upload_timestamp,
            protein_structure: metadata.protein_structure.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        pool.store_file(simple_model)?;
    }
    
    // Mantener backup en JSON como fallback
    let metadata_json = serde_json::to_string_pretty(metadata)?;
    let metadata_path = format!("./uploads/{}.metadata.json", metadata.file_id);
    
    let _ = web::block(move || -> anyhow::Result<()> {
        std::fs::write(&metadata_path, metadata_json)?;
        Ok(())
    })
    .await;
    
    Ok(())
}

async fn get_file_metadata(file_id: &str) -> anyhow::Result<Option<FileMetadata>> {
    let pool = get_database_pool()?;
    
    // Buscar en PostgreSQL primero
    if let Some(model) = ProteinFileModel::find_by_file_id(pool, file_id).await? {
        let protein_structure = model.get_protein_structure()?;
        
        return Ok(Some(FileMetadata {
            file_id: model.file_id,
            original_filename: model.original_filename,
            file_path: model.file_path,
            file_size: model.file_size as u64,
            content_type: model.content_type,
            upload_timestamp: model.upload_timestamp,
            protein_structure,
        }));
    }
    
    // Fallback a archivo JSON
    let metadata_path = format!("./uploads/{}.metadata.json", file_id);
    
    if !std::path::Path::new(&metadata_path).exists() {
        return Ok(None);
    }
    
    let metadata_json = web::block(move || -> anyhow::Result<String> {
        Ok(std::fs::read_to_string(&metadata_path)?)
    })
    .await??;
    
    let metadata: FileMetadata = serde_json::from_str(&metadata_json)?;
    Ok(Some(metadata))
}

async fn get_all_file_metadata(limit: usize, offset: usize) -> anyhow::Result<Vec<FileMetadata>> {
    let pool = get_database_pool()?;
    
    // Usar paginación real con PostgreSQL
    let models = ProteinFileModel::list_paginated(pool, limit as i64, offset as i64).await?;
    
    let mut files = Vec::new();
    for model in models {
        let protein_structure = model.get_protein_structure()?;
        
        files.push(FileMetadata {
            file_id: model.file_id,
            original_filename: model.original_filename,
            file_path: model.file_path,
            file_size: model.file_size as u64,
            content_type: model.content_type,
            upload_timestamp: model.upload_timestamp,
            protein_structure,
        });
    }
    
    Ok(files)
}

async fn delete_file_metadata(file_id: &str) -> anyhow::Result<()> {
    let pool = get_database_pool()?;
    
    // Eliminar de PostgreSQL
    ProteinFileModel::delete_by_file_id(pool, file_id).await?;
    
    // Eliminar backup JSON
    let metadata_path = format!("./uploads/{}.metadata.json", file_id);
    
    if std::path::Path::new(&metadata_path).exists() {
        let _ = web::block(move || -> anyhow::Result<()> {
            std::fs::remove_file(&metadata_path)?;
            Ok(())
        })
        .await;
    }
    
    Ok(())
}