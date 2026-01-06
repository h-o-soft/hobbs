//! Mail handlers for Web API.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;
use utoipa;

use crate::db::UserRepository;
use crate::mail::{MailRepository, MailUpdate, NewMail};
use crate::web::dto::{
    ApiResponse, AuthorInfo, MailDetailResponse, MailListResponse, PaginatedResponse,
    PaginationQuery, SendMailRequest, UnreadCountResponse,
};
use crate::web::error::ApiError;
use crate::web::handlers::AppState;
use crate::web::middleware::AuthUser;

/// GET /api/mail/inbox - List received mails.
#[utoipa::path(
    get,
    path = "/mail/inbox",
    tag = "mail",
    params(
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "List of received mails", body = Vec<MailListResponse>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_inbox(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<MailListResponse>>, ApiError> {
    let (offset, limit) = pagination.to_offset_limit();

    let (mails, total) = {
        let db = state.db.lock().await;
        let all_mails = MailRepository::list_inbox(db.conn(), claims.sub).map_err(|e| {
            tracing::error!("Failed to list inbox: {}", e);
            ApiError::internal("Failed to list inbox")
        })?;

        let total = all_mails.len() as i64;

        // Manual pagination
        let mails: Vec<_> = all_mails
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        (mails, total)
    };

    // Get user info for senders and recipients
    let responses = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        mails
            .into_iter()
            .map(|m| {
                let sender = user_repo
                    .get_by_id(m.sender_id)
                    .ok()
                    .flatten()
                    .map(|u| AuthorInfo {
                        id: u.id,
                        username: u.username,
                        nickname: u.nickname,
                    })
                    .unwrap_or_else(|| AuthorInfo {
                        id: m.sender_id,
                        username: "unknown".to_string(),
                        nickname: "Unknown".to_string(),
                    });

                let recipient = user_repo
                    .get_by_id(m.recipient_id)
                    .ok()
                    .flatten()
                    .map(|u| AuthorInfo {
                        id: u.id,
                        username: u.username,
                        nickname: u.nickname,
                    })
                    .unwrap_or_else(|| AuthorInfo {
                        id: m.recipient_id,
                        username: "unknown".to_string(),
                        nickname: "Unknown".to_string(),
                    });

                MailListResponse {
                    id: m.id,
                    sender,
                    recipient,
                    subject: m.subject,
                    is_read: m.is_read,
                    created_at: m.created_at.to_rfc3339(),
                }
            })
            .collect()
    };

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// GET /api/mail/sent - List sent mails.
#[utoipa::path(
    get,
    path = "/mail/sent",
    tag = "mail",
    params(
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "List of sent mails", body = Vec<MailListResponse>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_sent(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<MailListResponse>>, ApiError> {
    let (offset, limit) = pagination.to_offset_limit();

    let (mails, total) = {
        let db = state.db.lock().await;
        let all_mails = MailRepository::list_sent(db.conn(), claims.sub).map_err(|e| {
            tracing::error!("Failed to list sent: {}", e);
            ApiError::internal("Failed to list sent mails")
        })?;

        let total = all_mails.len() as i64;

        // Manual pagination
        let mails: Vec<_> = all_mails
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        (mails, total)
    };

    // Get user info for senders and recipients
    let responses = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        mails
            .into_iter()
            .map(|m| {
                let sender = user_repo
                    .get_by_id(m.sender_id)
                    .ok()
                    .flatten()
                    .map(|u| AuthorInfo {
                        id: u.id,
                        username: u.username,
                        nickname: u.nickname,
                    })
                    .unwrap_or_else(|| AuthorInfo {
                        id: m.sender_id,
                        username: "unknown".to_string(),
                        nickname: "Unknown".to_string(),
                    });

                let recipient = user_repo
                    .get_by_id(m.recipient_id)
                    .ok()
                    .flatten()
                    .map(|u| AuthorInfo {
                        id: u.id,
                        username: u.username,
                        nickname: u.nickname,
                    })
                    .unwrap_or_else(|| AuthorInfo {
                        id: m.recipient_id,
                        username: "unknown".to_string(),
                        nickname: "Unknown".to_string(),
                    });

                MailListResponse {
                    id: m.id,
                    sender,
                    recipient,
                    subject: m.subject,
                    is_read: m.is_read,
                    created_at: m.created_at.to_rfc3339(),
                }
            })
            .collect()
    };

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// GET /api/mail/:id - Get mail details.
#[utoipa::path(
    get,
    path = "/mail/{id}",
    tag = "mail",
    params(
        ("id" = i64, Path, description = "Mail ID")
    ),
    responses(
        (status = 200, description = "Mail details", body = MailDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Mail not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_mail(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(mail_id): Path<i64>,
) -> Result<Json<ApiResponse<MailDetailResponse>>, ApiError> {
    let (mail, sender, recipient) = {
        let db = state.db.lock().await;

        let mail = MailRepository::get_by_id(db.conn(), mail_id)
            .map_err(|e| {
                tracing::error!("Failed to get mail: {}", e);
                ApiError::internal("Failed to get mail")
            })?
            .ok_or_else(|| ApiError::not_found("Mail not found"))?;

        // Check access - must be sender or recipient
        if mail.sender_id != claims.sub && mail.recipient_id != claims.sub {
            return Err(ApiError::forbidden("Access denied"));
        }

        // Check if deleted
        if mail.sender_id == claims.sub && mail.is_deleted_by_sender {
            return Err(ApiError::not_found("Mail not found"));
        }
        if mail.recipient_id == claims.sub && mail.is_deleted_by_recipient {
            return Err(ApiError::not_found("Mail not found"));
        }

        // Mark as read if recipient
        if mail.recipient_id == claims.sub && !mail.is_read {
            MailRepository::update(db.conn(), mail_id, &MailUpdate::new().mark_as_read()).map_err(
                |e| {
                    tracing::error!("Failed to mark mail as read: {}", e);
                    ApiError::internal("Database error")
                },
            )?;
        }

        let user_repo = UserRepository::new(&*db);

        let sender = user_repo
            .get_by_id(mail.sender_id)
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: mail.sender_id,
                username: "unknown".to_string(),
                nickname: "Unknown".to_string(),
            });

        let recipient = user_repo
            .get_by_id(mail.recipient_id)
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: mail.recipient_id,
                username: "unknown".to_string(),
                nickname: "Unknown".to_string(),
            });

        (mail, sender, recipient)
    };

    let response = MailDetailResponse {
        id: mail.id,
        sender,
        recipient,
        subject: mail.subject,
        body: mail.body,
        is_read: mail.is_read,
        created_at: mail.created_at.to_rfc3339(),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// POST /api/mail - Send a mail.
#[utoipa::path(
    post,
    path = "/mail",
    tag = "mail",
    request_body = SendMailRequest,
    responses(
        (status = 200, description = "Mail sent", body = MailDetailResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Recipient not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn send_mail(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<SendMailRequest>,
) -> Result<Json<ApiResponse<MailDetailResponse>>, ApiError> {
    // Validate input
    if req.subject.trim().is_empty() {
        return Err(ApiError::bad_request("Subject is required"));
    }
    if req.body.trim().is_empty() {
        return Err(ApiError::bad_request("Body is required"));
    }

    let (mail, sender, recipient_info) = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        // Find recipient by username or ID
        let recipient = if let Ok(id) = req.recipient.parse::<i64>() {
            user_repo
                .get_by_id(id)
                .map_err(|e| {
                    tracing::error!("Failed to find recipient: {}", e);
                    ApiError::internal("Database error")
                })?
                .ok_or_else(|| ApiError::not_found("Recipient not found"))?
        } else {
            user_repo
                .get_by_username(&req.recipient)
                .map_err(|e| {
                    tracing::error!("Failed to find recipient: {}", e);
                    ApiError::internal("Database error")
                })?
                .ok_or_else(|| ApiError::not_found("Recipient not found"))?
        };

        // Cannot send to self
        if recipient.id == claims.sub {
            return Err(ApiError::bad_request("Cannot send mail to yourself"));
        }

        // Create mail
        let new_mail = NewMail::new(claims.sub, recipient.id, &req.subject, &req.body);
        let mail = MailRepository::create(db.conn(), &new_mail).map_err(|e| {
            tracing::error!("Failed to send mail: {}", e);
            ApiError::internal("Failed to send mail")
        })?;

        // Get sender info
        let sender = user_repo
            .get_by_id(claims.sub)
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
            });

        let recipient_info = AuthorInfo {
            id: recipient.id,
            username: recipient.username,
            nickname: recipient.nickname,
        };

        (mail, sender, recipient_info)
    };

    let response = MailDetailResponse {
        id: mail.id,
        sender,
        recipient: recipient_info,
        subject: mail.subject,
        body: mail.body,
        is_read: mail.is_read,
        created_at: mail.created_at.to_rfc3339(),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// DELETE /api/mail/:id - Delete a mail.
#[utoipa::path(
    delete,
    path = "/mail/{id}",
    tag = "mail",
    params(
        ("id" = i64, Path, description = "Mail ID")
    ),
    responses(
        (status = 200, description = "Mail deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Mail not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_mail(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(mail_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    {
        let db = state.db.lock().await;

        let mail = MailRepository::get_by_id(db.conn(), mail_id)
            .map_err(|e| {
                tracing::error!("Failed to get mail: {}", e);
                ApiError::internal("Failed to get mail")
            })?
            .ok_or_else(|| ApiError::not_found("Mail not found"))?;

        // Check access - must be sender or recipient
        if mail.sender_id != claims.sub && mail.recipient_id != claims.sub {
            return Err(ApiError::forbidden("Access denied"));
        }

        // Mark as deleted for the appropriate party
        let update = if mail.sender_id == claims.sub {
            MailUpdate::new().delete_by_sender()
        } else {
            MailUpdate::new().delete_by_recipient()
        };

        MailRepository::update(db.conn(), mail_id, &update).map_err(|e| {
            tracing::error!("Failed to delete mail: {}", e);
            ApiError::internal("Failed to delete mail")
        })?;
    }

    Ok(Json(ApiResponse::new(())))
}

/// GET /api/mail/unread-count - Get unread mail count.
#[utoipa::path(
    get,
    path = "/mail/unread-count",
    tag = "mail",
    responses(
        (status = 200, description = "Unread mail count", body = UnreadCountResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_unread_count(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<UnreadCountResponse>>, ApiError> {
    let count = {
        let db = state.db.lock().await;
        MailRepository::count_unread(db.conn(), claims.sub).map_err(|e| {
            tracing::error!("Failed to count unread: {}", e);
            ApiError::internal("Failed to count unread mails")
        })?
    };

    Ok(Json(ApiResponse::new(UnreadCountResponse {
        count: count as u64,
    })))
}
