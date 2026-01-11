//! File handlers for Web API.

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::header,
    response::Response,
    Json,
};
use std::sync::Arc;
use utoipa;

use crate::datetime::to_rfc3339;
use crate::db::{OneTimeTokenRepository, Role, TokenPurpose, UserRepository};
use crate::file::{FileRepository, FolderRepository, NewFile};
use crate::web::dto::{
    ApiResponse, AuthorInfo, FileResponse, FileUploadResponse, FolderResponse, PaginatedResponse,
    PaginationQuery,
};
use crate::web::error::ApiError;
use crate::web::handlers::AppState;
use crate::web::middleware::AuthUser;

/// Query parameters for one-time token download.
#[derive(Debug, serde::Deserialize)]
pub struct DownloadTokenQuery {
    /// One-time token for authentication.
    pub token: String,
}

/// Generate a safe Content-Disposition header value for file downloads.
///
/// This function sanitizes the filename to prevent header injection attacks
/// and uses RFC 5987 encoding for non-ASCII filenames.
///
/// # Security
///
/// The function:
/// - Removes control characters (including CR, LF which could cause header injection)
/// - Escapes double quotes and backslashes
/// - Uses RFC 5987 filename* parameter for proper Unicode support
fn content_disposition_header(filename: &str) -> String {
    // Sanitize filename for the basic filename parameter (ASCII fallback)
    let sanitized: String = filename
        .chars()
        .filter(|c| !c.is_control()) // Remove control characters (CR, LF, etc.)
        .map(|c| match c {
            '"' => '_',  // Replace double quotes
            '\\' => '_', // Replace backslashes
            _ => c,
        })
        .collect();

    // For ASCII-only filenames, use simple format
    if filename.is_ascii() && !filename.chars().any(|c| c.is_control() || c == '"' || c == '\\') {
        return format!("attachment; filename=\"{}\"", filename);
    }

    // Use RFC 5987 encoding for non-ASCII or special characters
    // filename* parameter with UTF-8 encoding
    let encoded = urlencoding::encode(filename);

    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        sanitized, encoded
    )
}

/// GET /api/folders - List all accessible folders.
#[utoipa::path(
    get,
    path = "/folders",
    tag = "folders",
    responses(
        (status = 200, description = "List of accessible folders", body = Vec<FolderResponse>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_folders(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<Vec<FolderResponse>>>, ApiError> {
    let user_role: Role = claims.role.parse().unwrap_or(Role::Guest);

    let folder_repo = FolderRepository::new(state.db.pool());

    // Get folders accessible by the user's role
    let folders = folder_repo.list_accessible(user_role).await.map_err(|e| {
        tracing::error!("Failed to list folders: {}", e);
        ApiError::internal("Failed to list folders")
    })?;

    let responses = {
        let file_repo = FileRepository::new(state.db.pool());

        let mut result = Vec::new();
        for f in folders {
            let file_count = file_repo.count_by_folder(f.id).await.unwrap_or(0);
            let can_upload = user_role >= f.upload_perm;

            result.push(FolderResponse {
                id: f.id,
                name: f.name,
                description: f.description,
                parent_id: f.parent_id,
                can_read: true, // Already filtered
                can_upload,
                file_count,
                created_at: to_rfc3339(&f.created_at),
            });
        }
        result
    };

    Ok(Json(ApiResponse::new(responses)))
}

/// GET /api/folders/:id - Get folder details.
#[utoipa::path(
    get,
    path = "/folders/{id}",
    tag = "folders",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    responses(
        (status = 200, description = "Folder details", body = FolderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Folder not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_folder(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(folder_id): Path<i64>,
) -> Result<Json<ApiResponse<FolderResponse>>, ApiError> {
    let user_role: Role = claims.role.parse().unwrap_or(Role::Guest);

    let folder_repo = FolderRepository::new(state.db.pool());
    let folder = folder_repo
        .get_by_id(folder_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get folder: {}", e);
            ApiError::internal("Failed to get folder")
        })?
        .ok_or_else(|| ApiError::not_found("Folder not found"))?;

    // Check read permission
    if user_role < folder.permission {
        return Err(ApiError::forbidden("Access denied"));
    }

    let file_repo = FileRepository::new(state.db.pool());
    let file_count = file_repo.count_by_folder(folder_id).await.unwrap_or(0);

    let can_upload = user_role >= folder.upload_perm;

    let response = FolderResponse {
        id: folder.id,
        name: folder.name,
        description: folder.description,
        parent_id: folder.parent_id,
        can_read: true,
        can_upload,
        file_count,
        created_at: to_rfc3339(&folder.created_at),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/folders/:id/files - List files in a folder.
#[utoipa::path(
    get,
    path = "/folders/{id}/files",
    tag = "files",
    params(
        ("id" = i64, Path, description = "Folder ID"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "List of files in folder", body = Vec<FileResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Folder not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(folder_id): Path<i64>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<FileResponse>>, ApiError> {
    let user_role: Role = claims.role.parse().unwrap_or(Role::Guest);
    let (offset, limit) = pagination.to_offset_limit();

    let (files, total, folder) = {
        let folder_repo = FolderRepository::new(state.db.pool());
        let file_repo = FileRepository::new(state.db.pool());

        let folder = folder_repo.get_by_id(folder_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get folder: {}", e);
                ApiError::internal("Failed to get folder")
            })?
            .ok_or_else(|| ApiError::not_found("Folder not found"))?;

        // Check read permission
        if user_role < folder.permission {
            return Err(ApiError::forbidden("Access denied"));
        }

        let all_files = file_repo.list_by_folder(folder_id).await.map_err(|e| {
            tracing::error!("Failed to list files: {}", e);
            ApiError::internal("Failed to list files")
        })?;

        let total = all_files.len() as i64;

        // Manual pagination
        let files: Vec<_> = all_files
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        (files, total, folder)
    };

    // Get user info for uploaders
    let responses = {
        let user_repo = UserRepository::new(state.db.pool());

        let mut result = Vec::new();
        for f in files {
            let uploader = user_repo
                .get_by_id(f.uploader_id)
                .await
                .ok()
                .flatten()
                .map(|u| AuthorInfo {
                    id: u.id,
                    username: u.username,
                    nickname: u.nickname,
                })
                .unwrap_or_else(|| AuthorInfo {
                    id: f.uploader_id,
                    username: "unknown".to_string(),
                    nickname: "Unknown".to_string(),
                });

            result.push(FileResponse {
                id: f.id,
                folder_id: f.folder_id,
                filename: f.filename,
                size: f.size,
                description: f.description,
                uploader,
                downloads: f.downloads,
                created_at: to_rfc3339(&f.created_at),
            });
        }
        result
    };

    // Suppress unused variable warning
    let _ = folder;

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// POST /api/folders/:id/files - Upload a file.
///
/// Request body: multipart/form-data with "file" and optional "description" fields.
#[utoipa::path(
    post,
    path = "/folders/{id}/files",
    tag = "files",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    responses(
        (status = 200, description = "File uploaded", body = FileUploadResponse),
        (status = 400, description = "Invalid input or file too large"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Upload permission denied"),
        (status = 404, description = "Folder not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(folder_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<FileUploadResponse>>, ApiError> {
    let user_role: Role = claims.role.parse().unwrap_or(Role::Guest);

    // Check if file storage is available
    let storage = state
        .file_storage
        .as_ref()
        .ok_or_else(|| ApiError::internal("File storage not configured"))?;

    // Check folder and permissions
    let folder_repo = FolderRepository::new(state.db.pool());
    let folder = folder_repo
        .get_by_id(folder_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get folder: {}", e);
            ApiError::internal("Failed to get folder")
        })?
        .ok_or_else(|| ApiError::not_found("Folder not found"))?;

    // Check upload permission
    if user_role < folder.upload_perm {
        return Err(ApiError::forbidden("Upload permission denied"));
    }

    // Extract file from multipart
    let mut filename: Option<String> = None;
    let mut description: Option<String> = None;
    let mut content: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to read multipart field: {}", e);
        ApiError::bad_request("Invalid multipart data")
    })? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                filename = field.file_name().map(|s| s.to_string());
                content = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to read file content: {}", e);
                            ApiError::bad_request("Failed to read file")
                        })?
                        .to_vec(),
                );
            }
            "description" => {
                description = Some(field.text().await.map_err(|e| {
                    tracing::error!("Failed to read description: {}", e);
                    ApiError::bad_request("Invalid description")
                })?);
            }
            _ => {}
        }
    }

    let filename = filename.ok_or_else(|| ApiError::bad_request("No file provided"))?;
    let content = content.ok_or_else(|| ApiError::bad_request("No file content"))?;

    // Check file size
    if content.len() as u64 > state.max_upload_size {
        let max_mb = state.max_upload_size / 1024 / 1024;
        return Err(ApiError::bad_request(&format!(
            "File too large (max {}MB)",
            max_mb
        )));
    }

    // Save file to storage
    let stored_name = storage.save(&content, &filename).map_err(|e| {
        tracing::error!("Failed to save file: {}", e);
        ApiError::internal("Failed to save file")
    })?;

    // Create file metadata
    let file = {
        let file_repo = FileRepository::new(state.db.pool());

        let mut new_file = NewFile::new(
            folder_id,
            &filename,
            &stored_name,
            content.len() as i64,
            claims.sub,
        );

        if let Some(ref desc) = description {
            if !desc.trim().is_empty() {
                new_file = new_file.with_description(desc);
            }
        }

        file_repo.create(&new_file).await.map_err(|e| {
            tracing::error!("Failed to create file metadata: {}", e);
            // Try to clean up the stored file
            let _ = storage.delete(&stored_name);
            ApiError::internal("Failed to create file")
        })?
    };

    // Get uploader info
    let uploader = {
        let user_repo = UserRepository::new(state.db.pool());

        user_repo
            .get_by_id(claims.sub)
            .await
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: claims.sub,
                username: claims.username.clone(),
                nickname: claims.username.clone(),
            })
    };

    let response = FileUploadResponse {
        file: FileResponse {
            id: file.id,
            folder_id: file.folder_id,
            filename: file.filename,
            size: file.size,
            description: file.description,
            uploader,
            downloads: file.downloads,
            created_at: to_rfc3339(&file.created_at),
        },
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/files/:id - Get file metadata.
#[utoipa::path(
    get,
    path = "/files/{id}",
    tag = "files",
    params(
        ("id" = i64, Path, description = "File ID")
    ),
    responses(
        (status = 200, description = "File metadata", body = FileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "File not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_file(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(file_id): Path<i64>,
) -> Result<Json<ApiResponse<FileResponse>>, ApiError> {
    let user_role: Role = claims.role.parse().unwrap_or(Role::Guest);

    let file_repo = FileRepository::new(state.db.pool());
    let folder_repo = FolderRepository::new(state.db.pool());

    let file = file_repo
        .get_by_id(file_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get file: {}", e);
            ApiError::internal("Failed to get file")
        })?
        .ok_or_else(|| ApiError::not_found("File not found"))?;

    let folder = folder_repo
        .get_by_id(file.folder_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get folder: {}", e);
            ApiError::internal("Failed to get folder")
        })?
        .ok_or_else(|| ApiError::not_found("Folder not found"))?;

    // Check read permission
    if user_role < folder.permission {
        return Err(ApiError::forbidden("Access denied"));
    }

    // Get uploader info
    let uploader = {
        let user_repo = UserRepository::new(state.db.pool());

        user_repo
            .get_by_id(file.uploader_id)
            .await
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: file.uploader_id,
                username: "unknown".to_string(),
                nickname: "Unknown".to_string(),
            })
    };

    let response = FileResponse {
        id: file.id,
        folder_id: file.folder_id,
        filename: file.filename,
        size: file.size,
        description: file.description,
        uploader,
        downloads: file.downloads,
        created_at: to_rfc3339(&file.created_at),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/files/:id/download - Download a file.
#[utoipa::path(
    get,
    path = "/files/{id}/download",
    tag = "files",
    params(
        ("id" = i64, Path, description = "File ID")
    ),
    responses(
        (status = 200, description = "File content", content_type = "application/octet-stream"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "File not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(file_id): Path<i64>,
) -> Result<Response<Body>, ApiError> {
    let user_role: Role = claims.role.parse().unwrap_or(Role::Guest);

    // Check if file storage is available
    let storage = state
        .file_storage
        .as_ref()
        .ok_or_else(|| ApiError::internal("File storage not configured"))?;

    let file_repo = FileRepository::new(state.db.pool());
    let folder_repo = FolderRepository::new(state.db.pool());

    let file = file_repo
        .get_by_id(file_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get file: {}", e);
            ApiError::internal("Failed to get file")
        })?
        .ok_or_else(|| ApiError::not_found("File not found"))?;

    let folder = folder_repo
        .get_by_id(file.folder_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get folder: {}", e);
            ApiError::internal("Failed to get folder")
        })?
        .ok_or_else(|| ApiError::not_found("Folder not found"))?;

    // Check read permission
    if user_role < folder.permission {
        return Err(ApiError::forbidden("Access denied"));
    }

    // Load file content
    let content = storage.load(&file.stored_name).map_err(|e| {
        tracing::error!("Failed to load file: {}", e);
        ApiError::internal("Failed to load file")
    })?;

    // Increment download count
    let _ = file_repo.increment_downloads(file_id).await;

    // Determine content type
    let content_type = mime_guess::from_path(&file.filename)
        .first_or_octet_stream()
        .to_string();

    // Build response with headers
    let response = Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            content_disposition_header(&file.filename),
        )
        .header(header::CONTENT_LENGTH, content.len())
        .body(Body::from(content))
        .map_err(|e| {
            tracing::error!("Failed to build response: {}", e);
            ApiError::internal("Failed to build response")
        })?;

    Ok(response)
}

/// GET /api/files/:id/download-with-token - Download a file using one-time token.
///
/// This endpoint is for browser direct downloads where Authorization headers cannot be used.
/// The token must be obtained from POST /api/auth/one-time-token with purpose "download"
/// and target_id set to the file ID.
#[utoipa::path(
    get,
    path = "/files/{id}/download-with-token",
    tag = "files",
    params(
        ("id" = i64, Path, description = "File ID"),
        ("token" = String, Query, description = "One-time token")
    ),
    responses(
        (status = 200, description = "File content", content_type = "application/octet-stream"),
        (status = 401, description = "Invalid or expired token"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "File not found")
    )
)]
pub async fn download_file_with_token(
    State(state): State<Arc<AppState>>,
    Path(file_id): Path<i64>,
    Query(query): Query<DownloadTokenQuery>,
) -> Result<Response<Body>, ApiError> {
    // Validate one-time token
    let repo = OneTimeTokenRepository::new(state.db.pool());
    let token_data = repo
        .consume_token(&query.token, TokenPurpose::Download, Some(file_id))
        .await
        .map_err(|e| {
            tracing::error!("Failed to validate token: {}", e);
            ApiError::internal("Token validation failed")
        })?
        .ok_or_else(|| ApiError::unauthorized("Invalid or expired token"))?;

    // Get user role
    let user_repo = UserRepository::new(state.db.pool());
    let user = user_repo
        .get_by_id(token_data.user_id)
        .await
        .map_err(|_| ApiError::internal("Database error"))?
        .ok_or_else(|| ApiError::unauthorized("User not found"))?;

    let user_role = user.role;

    // Check if file storage is available
    let storage = state
        .file_storage
        .as_ref()
        .ok_or_else(|| ApiError::internal("File storage not configured"))?;

    let file_repo = FileRepository::new(state.db.pool());
    let folder_repo = FolderRepository::new(state.db.pool());

    let file = file_repo
        .get_by_id(file_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get file: {}", e);
            ApiError::internal("Failed to get file")
        })?
        .ok_or_else(|| ApiError::not_found("File not found"))?;

    let folder = folder_repo
        .get_by_id(file.folder_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get folder: {}", e);
            ApiError::internal("Failed to get folder")
        })?
        .ok_or_else(|| ApiError::not_found("Folder not found"))?;

    // Check read permission
    if user_role < folder.permission {
        return Err(ApiError::forbidden("Access denied"));
    }

    // Load file content
    let content = storage.load(&file.stored_name).map_err(|e| {
        tracing::error!("Failed to load file: {}", e);
        ApiError::internal("Failed to load file")
    })?;

    // Increment download count
    let _ = file_repo.increment_downloads(file_id).await;

    // Determine content type
    let content_type = mime_guess::from_path(&file.filename)
        .first_or_octet_stream()
        .to_string();

    // Build response with headers
    let response = Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            content_disposition_header(&file.filename),
        )
        .header(header::CONTENT_LENGTH, content.len())
        .body(Body::from(content))
        .map_err(|e| {
            tracing::error!("Failed to build response: {}", e);
            ApiError::internal("Failed to build response")
        })?;

    Ok(response)
}

/// DELETE /api/files/:id - Delete a file.
#[utoipa::path(
    delete,
    path = "/files/{id}",
    tag = "files",
    params(
        ("id" = i64, Path, description = "File ID")
    ),
    responses(
        (status = 200, description = "File deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Delete permission denied"),
        (status = 404, description = "File not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(file_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let user_role: Role = claims.role.parse().unwrap_or(Role::Guest);

    // Check if file storage is available
    let storage = state
        .file_storage
        .as_ref()
        .ok_or_else(|| ApiError::internal("File storage not configured"))?;

    let file_repo = FileRepository::new(state.db.pool());

    let file = file_repo
        .get_by_id(file_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get file: {}", e);
            ApiError::internal("Failed to get file")
        })?
        .ok_or_else(|| ApiError::not_found("File not found"))?;

    // Check delete permission: uploader or SubOp+
    let can_delete = file.uploader_id == claims.sub || user_role >= Role::SubOp;

    if !can_delete {
        return Err(ApiError::forbidden("Delete permission denied"));
    }

    // Delete physical file
    let _ = storage.delete(&file.stored_name);

    // Delete metadata
    file_repo.delete(file_id).await.map_err(|e| {
        tracing::error!("Failed to delete file metadata: {}", e);
        ApiError::internal("Failed to delete file")
    })?;

    Ok(Json(ApiResponse::new(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_disposition_header_simple_ascii() {
        let result = content_disposition_header("document.txt");
        assert_eq!(result, "attachment; filename=\"document.txt\"");
    }

    #[test]
    fn test_content_disposition_header_with_spaces() {
        let result = content_disposition_header("my document.txt");
        assert_eq!(result, "attachment; filename=\"my document.txt\"");
    }

    #[test]
    fn test_content_disposition_header_japanese() {
        let result = content_disposition_header("日本語ファイル.txt");
        assert!(result.starts_with("attachment; filename=\""));
        assert!(result.contains("filename*=UTF-8''"));
        // Check that the encoded version is present
        assert!(result.contains("%E6%97%A5%E6%9C%AC%E8%AA%9E"));
    }

    #[test]
    fn test_content_disposition_header_double_quote() {
        let result = content_disposition_header("test\"file.txt");
        // Should sanitize the quote in the fallback filename
        assert!(result.contains("filename=\"test_file.txt\""));
        // And encode it in filename*
        assert!(result.contains("filename*=UTF-8''"));
        assert!(result.contains("%22")); // URL-encoded double quote
    }

    #[test]
    fn test_content_disposition_header_backslash() {
        let result = content_disposition_header("test\\file.txt");
        // Should sanitize the backslash in the fallback filename
        assert!(result.contains("filename=\"test_file.txt\""));
        // And encode it in filename*
        assert!(result.contains("filename*=UTF-8''"));
    }

    #[test]
    fn test_content_disposition_header_control_characters() {
        // Test with carriage return and line feed (header injection attempt)
        let result = content_disposition_header("test\r\nX-Injected: bad.txt");
        // Control characters should be removed
        assert!(!result.contains('\r'));
        assert!(!result.contains('\n'));
        // Should still produce valid output
        assert!(result.starts_with("attachment; filename="));
    }

    #[test]
    fn test_content_disposition_header_null_character() {
        let result = content_disposition_header("test\x00null.txt");
        // Null character should be removed
        assert!(!result.contains('\x00'));
        assert!(result.starts_with("attachment; filename="));
    }

    #[test]
    fn test_content_disposition_header_mixed_attack() {
        // Complex attack vector
        let result = content_disposition_header("file\"\r\nX-Evil: header\r\n\r\n<script>.txt");
        // Should not contain any control characters
        assert!(!result.contains('\r'));
        assert!(!result.contains('\n'));
        // Should still be a valid header
        assert!(result.starts_with("attachment; filename="));
    }
}
