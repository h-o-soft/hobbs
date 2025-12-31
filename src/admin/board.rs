//! Board management for administrators.
//!
//! This module provides administrative functions for managing boards:
//! - Create board (SubOp and above)
//! - Update board (SubOp and above)
//! - Delete board (SysOp only)

use crate::auth::require_sysop;
use crate::board::{Board, BoardRepository, BoardUpdate, NewBoard};
use crate::db::{Database, User};

use super::{require_admin, AdminError};

/// Request to create a new board.
#[derive(Debug, Clone)]
pub struct CreateBoardRequest {
    /// Board name.
    pub name: String,
    /// Board data.
    pub board: NewBoard,
}

impl CreateBoardRequest {
    /// Create a new CreateBoardRequest.
    pub fn new(board: NewBoard) -> Self {
        Self {
            name: board.name.clone(),
            board,
        }
    }
}

/// Admin service for board management.
pub struct BoardAdminService<'a> {
    db: &'a Database,
}

impl<'a> BoardAdminService<'a> {
    /// Create a new BoardAdminService.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new board.
    ///
    /// Requires SubOp or higher permission.
    pub fn create_board(
        &self,
        request: &CreateBoardRequest,
        admin: &User,
    ) -> Result<Board, AdminError> {
        require_admin(Some(admin))?;

        let repo = BoardRepository::new(self.db);

        // Check if name already exists
        if repo.name_exists(&request.name)? {
            return Err(AdminError::InvalidOperation(format!(
                "掲示板名「{}」は既に使用されています",
                request.name
            )));
        }

        let board = repo.create(&request.board)?;
        Ok(board)
    }

    /// Update an existing board.
    ///
    /// Requires SubOp or higher permission.
    pub fn update_board(
        &self,
        board_id: i64,
        update: &BoardUpdate,
        admin: &User,
    ) -> Result<Board, AdminError> {
        require_admin(Some(admin))?;

        let repo = BoardRepository::new(self.db);

        // Check if board exists
        let existing = repo
            .get_by_id(board_id)?
            .ok_or_else(|| AdminError::NotFound("掲示板".to_string()))?;

        // If name is being changed, check for duplicates
        if let Some(ref new_name) = update.name {
            if *new_name != existing.name && repo.name_exists(new_name)? {
                return Err(AdminError::InvalidOperation(format!(
                    "掲示板名「{new_name}」は既に使用されています"
                )));
            }
        }

        let updated = repo
            .update(board_id, update)?
            .ok_or_else(|| AdminError::NotFound("掲示板".to_string()))?;

        Ok(updated)
    }

    /// Delete a board.
    ///
    /// Requires SysOp permission.
    /// This will also delete all threads and posts in the board.
    pub fn delete_board(&self, board_id: i64, admin: &User) -> Result<bool, AdminError> {
        require_sysop(Some(admin))?;

        let repo = BoardRepository::new(self.db);

        // Check if board exists
        repo.get_by_id(board_id)?
            .ok_or_else(|| AdminError::NotFound("掲示板".to_string()))?;

        let deleted = repo.delete(board_id)?;
        Ok(deleted)
    }

    /// Get a board by ID.
    ///
    /// Requires SubOp or higher permission to view all boards (including inactive).
    pub fn get_board(&self, board_id: i64, admin: &User) -> Result<Board, AdminError> {
        require_admin(Some(admin))?;

        let repo = BoardRepository::new(self.db);
        let board = repo
            .get_by_id(board_id)?
            .ok_or_else(|| AdminError::NotFound("掲示板".to_string()))?;

        Ok(board)
    }

    /// List all boards (including inactive).
    ///
    /// Requires SubOp or higher permission.
    pub fn list_all_boards(&self, admin: &User) -> Result<Vec<Board>, AdminError> {
        require_admin(Some(admin))?;

        let repo = BoardRepository::new(self.db);
        let boards = repo.list_all()?;
        Ok(boards)
    }

    /// Toggle board active status.
    ///
    /// Requires SubOp or higher permission.
    pub fn toggle_board_active(&self, board_id: i64, admin: &User) -> Result<Board, AdminError> {
        require_admin(Some(admin))?;

        let repo = BoardRepository::new(self.db);

        let existing = repo
            .get_by_id(board_id)?
            .ok_or_else(|| AdminError::NotFound("掲示板".to_string()))?;

        let update = BoardUpdate::new().is_active(!existing.is_active);
        let updated = repo
            .update(board_id, &update)?
            .ok_or_else(|| AdminError::NotFound("掲示板".to_string()))?;

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoardType;
    use crate::db::Role;
    use crate::server::CharacterEncoding;

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_user(id: i64, role: Role) -> User {
        User {
            id,
            username: format!("user{id}"),
            password: "hash".to_string(),
            nickname: format!("User {id}"),
            email: None,
            role,
            profile: None,
            terminal: "standard".to_string(),
            encoding: CharacterEncoding::default(),
            language: "en".to_string(),
            created_at: "2024-01-01".to_string(),
            last_login: None,
            is_active: true,
        }
    }

    #[test]
    fn test_create_board_as_subop() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let new_board = NewBoard::new("テスト掲示板")
            .with_description("テスト用の掲示板")
            .with_board_type(BoardType::Thread);

        let request = CreateBoardRequest::new(new_board);
        let board = service.create_board(&request, &subop).unwrap();

        assert_eq!(board.name, "テスト掲示板");
        assert_eq!(board.description, Some("テスト用の掲示板".to_string()));
        assert_eq!(board.board_type, BoardType::Thread);
    }

    #[test]
    fn test_create_board_as_sysop() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);

        let new_board = NewBoard::new("管理者掲示板");
        let request = CreateBoardRequest::new(new_board);
        let board = service.create_board(&request, &sysop).unwrap();

        assert_eq!(board.name, "管理者掲示板");
    }

    #[test]
    fn test_create_board_as_member_fails() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let member = create_test_user(1, Role::Member);

        let new_board = NewBoard::new("テスト");
        let request = CreateBoardRequest::new(new_board);
        let result = service.create_board(&request, &member);

        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_create_board_duplicate_name() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let new_board = NewBoard::new("テスト");
        let request = CreateBoardRequest::new(new_board);
        service.create_board(&request, &subop).unwrap();

        let new_board2 = NewBoard::new("テスト");
        let request2 = CreateBoardRequest::new(new_board2);
        let result = service.create_board(&request2, &subop);

        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    #[test]
    fn test_update_board() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let new_board = NewBoard::new("元の名前");
        let request = CreateBoardRequest::new(new_board);
        let board = service.create_board(&request, &subop).unwrap();

        let update = BoardUpdate::new()
            .name("新しい名前")
            .description(Some("新しい説明".to_string()));

        let updated = service.update_board(board.id, &update, &subop).unwrap();

        assert_eq!(updated.name, "新しい名前");
        assert_eq!(updated.description, Some("新しい説明".to_string()));
    }

    #[test]
    fn test_update_board_name_conflict() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let board1 = service
            .create_board(&CreateBoardRequest::new(NewBoard::new("掲示板1")), &subop)
            .unwrap();
        service
            .create_board(&CreateBoardRequest::new(NewBoard::new("掲示板2")), &subop)
            .unwrap();

        let update = BoardUpdate::new().name("掲示板2");
        let result = service.update_board(board1.id, &update, &subop);

        assert!(matches!(result, Err(AdminError::InvalidOperation(_))));
    }

    #[test]
    fn test_update_nonexistent_board() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let update = BoardUpdate::new().name("新しい名前");
        let result = service.update_board(999, &update, &subop);

        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_board_as_sysop() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);

        let new_board = NewBoard::new("削除対象");
        let request = CreateBoardRequest::new(new_board);
        let board = service.create_board(&request, &sysop).unwrap();

        let deleted = service.delete_board(board.id, &sysop).unwrap();
        assert!(deleted);

        let result = service.get_board(board.id, &sysop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_delete_board_as_subop_fails() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);
        let subop = create_test_user(2, Role::SubOp);

        let new_board = NewBoard::new("削除対象");
        let request = CreateBoardRequest::new(new_board);
        let board = service.create_board(&request, &sysop).unwrap();

        let result = service.delete_board(board.id, &subop);
        assert!(matches!(result, Err(AdminError::Permission(_))));
    }

    #[test]
    fn test_delete_nonexistent_board() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let sysop = create_test_user(1, Role::SysOp);

        let result = service.delete_board(999, &sysop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }

    #[test]
    fn test_list_all_boards() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        service
            .create_board(&CreateBoardRequest::new(NewBoard::new("掲示板1")), &subop)
            .unwrap();
        let board2 = service
            .create_board(&CreateBoardRequest::new(NewBoard::new("掲示板2")), &subop)
            .unwrap();
        service
            .create_board(&CreateBoardRequest::new(NewBoard::new("掲示板3")), &subop)
            .unwrap();

        // Deactivate one board
        service
            .update_board(board2.id, &BoardUpdate::new().is_active(false), &subop)
            .unwrap();

        let boards = service.list_all_boards(&subop).unwrap();
        assert_eq!(boards.len(), 3); // Should include inactive boards
    }

    #[test]
    fn test_toggle_board_active() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let board = service
            .create_board(&CreateBoardRequest::new(NewBoard::new("テスト")), &subop)
            .unwrap();
        assert!(board.is_active);

        let toggled = service.toggle_board_active(board.id, &subop).unwrap();
        assert!(!toggled.is_active);

        let toggled_again = service.toggle_board_active(board.id, &subop).unwrap();
        assert!(toggled_again.is_active);
    }

    #[test]
    fn test_get_board() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let created = service
            .create_board(&CreateBoardRequest::new(NewBoard::new("テスト")), &subop)
            .unwrap();

        let board = service.get_board(created.id, &subop).unwrap();
        assert_eq!(board.name, "テスト");
    }

    #[test]
    fn test_get_board_not_found() {
        let db = setup_db();
        let service = BoardAdminService::new(&db);
        let subop = create_test_user(1, Role::SubOp);

        let result = service.get_board(999, &subop);
        assert!(matches!(result, Err(AdminError::NotFound(_))));
    }
}
