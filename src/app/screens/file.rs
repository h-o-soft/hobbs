//! File screen handler.

use super::common::{Pagination, ScreenContext};
use super::ScreenResult;
use crate::db::UserRepository;
use crate::error::Result;
use crate::file::{FileRepository, FileService, FileStorage, FolderRepository};
use crate::server::TelnetSession;
use crate::file::UploadRequest;
use crate::xmodem::{xmodem_receive, xmodem_send, TransferError};

/// File screen handler.
pub struct FileScreen;

impl FileScreen {
    /// Run the file browser screen.
    pub async fn run_browser(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        folder_id: Option<i64>,
    ) -> Result<ScreenResult> {
        let mut current_folder = folder_id;
        let mut pagination = Pagination::new(1, 10, 0);

        loop {
            // Get folder info
            let folder_name = if let Some(fid) = current_folder {
                FolderRepository::get_by_id(ctx.db.conn(), fid)?
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| ctx.i18n.t("file.folder_list").to_string())
            } else {
                ctx.i18n.t("file.folder_list").to_string()
            };

            // Get child folders
            let child_folders = if let Some(fid) = current_folder {
                FolderRepository::list_by_parent(ctx.db.conn(), fid)?
            } else {
                FolderRepository::list_root(ctx.db.conn())?
            };

            // Get files
            let total = if let Some(fid) = current_folder {
                FileRepository::count_by_folder(ctx.db.conn(), fid)? as usize
            } else {
                0 // Root folder doesn't have files directly
            };
            pagination.total = total;

            let files = if let Some(fid) = current_folder {
                FileRepository::list_by_folder(ctx.db.conn(), fid)?
            } else {
                vec![]
            };

            // Display
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", folder_name))
                .await?;
            ctx.send_line(session, "").await?;

            // Show child folders
            if !child_folders.is_empty() {
                ctx.send_line(session, &format!("{}:", ctx.i18n.t("file.folder_list")))
                    .await?;
                for (i, folder) in child_folders.iter().enumerate() {
                    let letter = (b'A' + i as u8) as char;
                    ctx.send_line(session, &format!("  [{}] {}", letter, folder.name))
                        .await?;
                }
                ctx.send_line(session, "").await?;
            }

            // Show files
            if files.is_empty() && child_folders.is_empty() {
                ctx.send_line(session, ctx.i18n.t("file.no_files")).await?;
            } else if !files.is_empty() {
                ctx.send_line(session, &format!("{}:", ctx.i18n.t("file.file_list")))
                    .await?;
                ctx.send_line(
                    session,
                    &format!(
                        "  {:<4} {:<20} {:>10} {:>6}",
                        ctx.i18n.t("common.number"),
                        ctx.i18n.t("file.filename"),
                        ctx.i18n.t("file.size"),
                        ctx.i18n.t("file.download_count")
                    ),
                )
                .await?;
                ctx.send_line(session, &"-".repeat(50)).await?;

                for (i, file) in files.iter().enumerate() {
                    let num = pagination.offset() + i + 1;
                    let filename = if file.filename.chars().count() > 18 {
                        let truncated: String = file.filename.chars().take(15).collect();
                        format!("{}...", truncated)
                    } else {
                        file.filename.clone()
                    };
                    let size = Self::format_size(file.size);

                    ctx.send_line(
                        session,
                        &format!(
                            "  {:<4} {:<20} {:>10} {:>6}",
                            num, filename, size, file.downloads
                        ),
                    )
                    .await?;
                }
            }

            // Show pagination
            ctx.send_line(session, "").await?;
            if pagination.total > 0 {
                ctx.send_line(
                    session,
                    &ctx.i18n.t_with(
                        "board.page_of",
                        &[
                            ("current", &pagination.page.to_string()),
                            ("total", &pagination.total_pages().to_string()),
                        ],
                    ),
                )
                .await?;
            }

            // Prompt
            let mut prompt = format!(
                "[N]={} [P]={}",
                ctx.i18n.t("common.next"),
                ctx.i18n.t("common.previous")
            );
            if current_folder.is_some() {
                prompt.push_str(&format!(" [U]={}", ctx.i18n.t("file.upload")));
                prompt.push_str(&format!(" [B]={}", ctx.i18n.t("common.back")));
            }
            prompt.push_str(&format!(" [Q]={}: ", ctx.i18n.t("common.quit")));
            ctx.send(session, &prompt).await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" => return Ok(ScreenResult::Back),
                "" => {
                    if current_folder.is_none() {
                        return Ok(ScreenResult::Back);
                    }
                }
                "n" => pagination.next(),
                "p" => pagination.prev(),
                "b" => {
                    // Go back to parent folder
                    if let Some(fid) = current_folder {
                        let folder = FolderRepository::get_by_id(ctx.db.conn(), fid)?;
                        current_folder = folder.and_then(|f| f.parent_id);
                        pagination = Pagination::new(1, 10, 0);
                    }
                }
                "u" => {
                    // Upload file
                    if let Some(fid) = current_folder {
                        Self::upload_file(ctx, session, fid).await?;
                    }
                }
                _ => {
                    // Check if it's a folder letter
                    if input.len() == 1 {
                        let ch = input.chars().next().unwrap().to_ascii_uppercase();
                        if ch >= 'A' {
                            let idx = (ch as u8 - b'A') as usize;
                            if idx < child_folders.len() {
                                current_folder = Some(child_folders[idx].id);
                                pagination = Pagination::new(1, 10, 0);
                                continue;
                            }
                        }
                    }

                    // Check if it's a file number
                    if let Some(num) = ctx.parse_number(input) {
                        let idx = num as usize - 1;
                        if idx < files.len() {
                            Self::view_file(ctx, session, files[idx].id).await?;
                        }
                    }
                }
            }
        }
    }

    /// View file details.
    async fn view_file(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        file_id: i64,
    ) -> Result<()> {
        let file = match FileRepository::get_by_id(ctx.db.conn(), file_id)? {
            Some(f) => f,
            None => return Ok(()),
        };

        // Get uploader name
        let user_repo = UserRepository::new(&ctx.db);
        let uploader = user_repo
            .get_by_id(file.uploader_id)?
            .map(|u| u.nickname)
            .unwrap_or_else(|| "Unknown".to_string());

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", file.filename))
            .await?;
        ctx.send_line(
            session,
            &format!("{}: {}", ctx.i18n.t("file.filename"), file.filename),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("file.size"),
                Self::format_size(file.size)
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("{}: {}", ctx.i18n.t("file.uploaded_by"), uploader),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("file.uploaded_at"),
                file.created_at.format("%Y/%m/%d %H:%M")
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("{}: {}", ctx.i18n.t("file.download_count"), file.downloads),
        )
        .await?;

        if let Some(desc) = &file.description {
            ctx.send_line(session, &"-".repeat(40)).await?;
            ctx.send_line(session, desc).await?;
        }

        ctx.send_line(session, &"-".repeat(40)).await?;

        // Ask if user wants to download
        ctx.send(
            session,
            &format!(
                "{} [Y/N]: ",
                ctx.i18n.t("file.download_confirm")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        if input.trim().eq_ignore_ascii_case("y") {
            // Perform download
            Self::download_file(ctx, session, file_id).await?;
        }

        Ok(())
    }

    /// Download a file using XMODEM protocol.
    async fn download_file(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        file_id: i64,
    ) -> Result<()> {
        // Get current user
        let user_id = match session.user_id() {
            Some(id) => id,
            None => {
                ctx.send_line(session, ctx.i18n.t("error.not_logged_in"))
                    .await?;
                return Ok(());
            }
        };

        let user_repo = UserRepository::new(&ctx.db);
        let user = match user_repo.get_by_id(user_id)? {
            Some(u) => u,
            None => {
                ctx.send_line(session, ctx.i18n.t("error.user_not_found"))
                    .await?;
                return Ok(());
            }
        };

        // Get file storage path from config
        let storage = match FileStorage::new(&ctx.config.files.storage_path) {
            Ok(s) => s,
            Err(e) => {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("file.xmodem_failed"), e),
                )
                .await?;
                return Ok(());
            }
        };
        let file_service = FileService::new(&ctx.db, &storage);

        // Download file (this checks permissions and increments counter)
        let download_result = match file_service.download(file_id, &user) {
            Ok(result) => result,
            Err(e) => {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("file.xmodem_failed"), e),
                )
                .await?;
                return Ok(());
            }
        };

        // Notify user to start XMODEM receive
        ctx.send_line(session, "").await?;
        ctx.send_line(session, ctx.i18n.t("file.xmodem_start_download"))
            .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {} ({})",
                ctx.i18n.t("file.filename"),
                download_result.metadata.filename,
                Self::format_size(download_result.metadata.size)
            ),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Small delay to let user start their XMODEM receiver
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Perform XMODEM transfer
        match xmodem_send(session.stream_mut(), &download_result.content).await {
            Ok(bytes_sent) => {
                ctx.send_line(session, "").await?;
                ctx.send_line(
                    session,
                    &format!(
                        "{} ({} bytes)",
                        ctx.i18n.t("file.xmodem_complete"),
                        bytes_sent
                    ),
                )
                .await?;
            }
            Err(e) => {
                ctx.send_line(session, "").await?;
                let error_msg = match e {
                    TransferError::Cancelled => ctx.i18n.t("file.xmodem_cancelled").to_string(),
                    TransferError::Timeout => ctx.i18n.t("file.xmodem_timeout").to_string(),
                    _ => format!("{}: {}", ctx.i18n.t("file.xmodem_failed"), e),
                };
                ctx.send_line(session, &error_msg).await?;
            }
        }

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Upload a file using XMODEM protocol.
    async fn upload_file(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        folder_id: i64,
    ) -> Result<()> {
        // Get current user
        let user_id = match session.user_id() {
            Some(id) => id,
            None => {
                ctx.send_line(session, ctx.i18n.t("error.not_logged_in"))
                    .await?;
                return Ok(());
            }
        };

        let user_repo = UserRepository::new(&ctx.db);
        let user = match user_repo.get_by_id(user_id)? {
            Some(u) => u,
            None => {
                ctx.send_line(session, ctx.i18n.t("error.user_not_found"))
                    .await?;
                return Ok(());
            }
        };

        // Get filename from user
        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!("{}: ", ctx.i18n.t("file.upload_prompt")),
        )
        .await?;
        let filename = ctx.read_line(session).await?;
        let filename = filename.trim();

        if filename.is_empty() {
            return Ok(());
        }

        // Get optional description
        ctx.send(
            session,
            &format!("{}: ", ctx.i18n.t("file.upload_description_prompt")),
        )
        .await?;
        let description = ctx.read_line(session).await?;
        let description = description.trim();
        let description = if description.is_empty() {
            None
        } else {
            Some(description.to_string())
        };

        // Notify user to start XMODEM send
        ctx.send_line(session, "").await?;
        ctx.send_line(session, ctx.i18n.t("file.xmodem_start_upload"))
            .await?;

        // Perform XMODEM receive
        match xmodem_receive(session.stream_mut()).await {
            Ok(data) => {
                // Save the file
                let storage = match FileStorage::new(&ctx.config.files.storage_path) {
                    Ok(s) => s,
                    Err(e) => {
                        ctx.send_line(
                            session,
                            &format!("{}: {}", ctx.i18n.t("file.upload_failed"), e),
                        )
                        .await?;
                        ctx.wait_for_enter(session).await?;
                        return Ok(());
                    }
                };
                let file_service = FileService::new(&ctx.db, &storage);

                let mut request = UploadRequest::new(folder_id, filename.to_string(), data);
                if let Some(desc) = description {
                    request = request.with_description(desc);
                }

                match file_service.upload(&request, &user) {
                    Ok(metadata) => {
                        ctx.send_line(session, "").await?;
                        ctx.send_line(
                            session,
                            &format!(
                                "{} ({} bytes)",
                                ctx.i18n.t("file.file_uploaded"),
                                metadata.size
                            ),
                        )
                        .await?;
                    }
                    Err(e) => {
                        ctx.send_line(session, "").await?;
                        ctx.send_line(
                            session,
                            &format!("{}: {}", ctx.i18n.t("file.upload_failed"), e),
                        )
                        .await?;
                    }
                }
            }
            Err(e) => {
                ctx.send_line(session, "").await?;
                let error_msg = match e {
                    TransferError::Cancelled => ctx.i18n.t("file.xmodem_cancelled").to_string(),
                    TransferError::Timeout => ctx.i18n.t("file.xmodem_timeout").to_string(),
                    _ => format!("{}: {}", ctx.i18n.t("file.xmodem_failed"), e),
                };
                ctx.send_line(session, &error_msg).await?;
            }
        }

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Format file size for display.
    fn format_size(size: i64) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if size >= GB {
            format!("{:.1} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.1} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.1} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_screen_exists() {
        let _ = FileScreen;
    }

    #[test]
    fn test_format_size() {
        assert_eq!(FileScreen::format_size(500), "500 B");
        assert_eq!(FileScreen::format_size(1024), "1.0 KB");
        assert_eq!(FileScreen::format_size(1536), "1.5 KB");
        assert_eq!(FileScreen::format_size(1048576), "1.0 MB");
        assert_eq!(FileScreen::format_size(1073741824), "1.0 GB");
    }
}
