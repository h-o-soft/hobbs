//! Board repository for HOBBS.
//!
//! This module provides CRUD operations for boards in the database.

use rusqlite::{params, Row};

use super::types::{Board, BoardType, BoardUpdate, NewBoard};
use crate::db::{Database, Role};
use crate::{HobbsError, Result};

/// Repository for board CRUD operations.
pub struct BoardRepository<'a> {
    db: &'a Database,
}

impl<'a> BoardRepository<'a> {
    /// Create a new BoardRepository with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new board in the database.
    ///
    /// Returns the created board with the assigned ID.
    pub fn create(&self, new_board: &NewBoard) -> Result<Board> {
        self.db.conn().execute(
            "INSERT INTO boards (name, description, board_type, min_read_role, min_write_role, sort_order)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                &new_board.name,
                &new_board.description,
                new_board.board_type.as_str(),
                new_board.min_read_role.as_str(),
                new_board.min_write_role.as_str(),
                new_board.sort_order,
            ],
        )?;

        let id = self.db.conn().last_insert_rowid();
        self.get_by_id(id)?
            .ok_or_else(|| HobbsError::NotFound("board".to_string()))
    }

    /// Get a board by ID.
    pub fn get_by_id(&self, id: i64) -> Result<Option<Board>> {
        let result = self.db.conn().query_row(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards WHERE id = ?",
            [id],
            Self::row_to_board,
        );

        match result {
            Ok(board) => Ok(Some(board)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a board by name.
    pub fn get_by_name(&self, name: &str) -> Result<Option<Board>> {
        let result = self.db.conn().query_row(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards WHERE name = ?",
            [name],
            Self::row_to_board,
        );

        match result {
            Ok(board) => Ok(Some(board)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update a board by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated board, or None if not found.
    pub fn update(&self, id: i64, update: &BoardUpdate) -> Result<Option<Board>> {
        if update.is_empty() {
            return self.get_by_id(id);
        }

        let mut fields = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref name) = update.name {
            fields.push("name = ?");
            values.push(Box::new(name.clone()));
        }
        if let Some(ref description) = update.description {
            fields.push("description = ?");
            values.push(Box::new(description.clone()));
        }
        if let Some(board_type) = update.board_type {
            fields.push("board_type = ?");
            values.push(Box::new(board_type.as_str().to_string()));
        }
        if let Some(role) = update.min_read_role {
            fields.push("min_read_role = ?");
            values.push(Box::new(role.as_str().to_string()));
        }
        if let Some(role) = update.min_write_role {
            fields.push("min_write_role = ?");
            values.push(Box::new(role.as_str().to_string()));
        }
        if let Some(sort_order) = update.sort_order {
            fields.push("sort_order = ?");
            values.push(Box::new(sort_order));
        }
        if let Some(is_active) = update.is_active {
            fields.push("is_active = ?");
            values.push(Box::new(if is_active { 1i64 } else { 0i64 }));
        }

        let sql = format!("UPDATE boards SET {} WHERE id = ?", fields.join(", "));
        values.push(Box::new(id));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        let affected = self.db.conn().execute(&sql, params.as_slice())?;

        if affected == 0 {
            return Ok(None);
        }

        self.get_by_id(id)
    }

    /// Delete a board by ID.
    ///
    /// Returns true if a board was deleted, false if not found.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM boards WHERE id = ?", [id])?;
        Ok(affected > 0)
    }

    /// List all active boards, ordered by sort_order then created_at.
    pub fn list_active(&self) -> Result<Vec<Board>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards WHERE is_active = 1 ORDER BY sort_order ASC, created_at ASC, id ASC",
        )?;

        let boards = stmt
            .query_map([], Self::row_to_board)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(boards)
    }

    /// List all boards (including inactive), ordered by sort_order then created_at.
    pub fn list_all(&self) -> Result<Vec<Board>> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards ORDER BY sort_order ASC, created_at ASC, id ASC",
        )?;

        let boards = stmt
            .query_map([], Self::row_to_board)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(boards)
    }

    /// List boards accessible by a user with the given role.
    ///
    /// Only returns active boards where min_read_role <= user's role.
    pub fn list_accessible(&self, user_role: Role) -> Result<Vec<Board>> {
        let all_boards = self.list_active()?;
        let accessible = all_boards
            .into_iter()
            .filter(|b| b.can_read(user_role))
            .collect();
        Ok(accessible)
    }

    /// List boards writable by a user with the given role.
    ///
    /// Only returns active boards where min_write_role <= user's role.
    pub fn list_writable(&self, user_role: Role) -> Result<Vec<Board>> {
        let all_boards = self.list_active()?;
        let writable = all_boards
            .into_iter()
            .filter(|b| b.can_write(user_role))
            .collect();
        Ok(writable)
    }

    /// Count all boards.
    pub fn count(&self) -> Result<i64> {
        let count: i64 = self
            .db
            .conn()
            .query_row("SELECT COUNT(*) FROM boards", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Count active boards.
    pub fn count_active(&self) -> Result<i64> {
        let count: i64 = self.db.conn().query_row(
            "SELECT COUNT(*) FROM boards WHERE is_active = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Check if a board name is already taken.
    pub fn name_exists(&self, name: &str) -> Result<bool> {
        let exists: bool = self.db.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM boards WHERE name = ?)",
            [name],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Convert a database row to a Board struct.
    fn row_to_board(row: &Row<'_>) -> rusqlite::Result<Board> {
        let board_type_str: String = row.get(3)?;
        let board_type = board_type_str.parse().unwrap_or(BoardType::Thread);
        let min_read_role_str: String = row.get(4)?;
        let min_read_role = min_read_role_str.parse().unwrap_or(Role::Guest);
        let min_write_role_str: String = row.get(5)?;
        let min_write_role = min_write_role_str.parse().unwrap_or(Role::Member);
        let is_active: i64 = row.get(7)?;

        Ok(Board {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            board_type,
            min_read_role,
            min_write_role,
            sort_order: row.get(6)?,
            is_active: is_active != 0,
            created_at: row.get(8)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    #[test]
    fn test_create_board() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).unwrap();

        assert_eq!(board.id, 1);
        assert_eq!(board.name, "general");
        assert_eq!(board.board_type, BoardType::Thread);
        assert_eq!(board.min_read_role, Role::Guest);
        assert_eq!(board.min_write_role, Role::Member);
        assert!(board.is_active);
    }

    #[test]
    fn test_create_board_with_options() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("admin-board")
            .with_description("For administrators only")
            .with_board_type(BoardType::Flat)
            .with_min_read_role(Role::SubOp)
            .with_min_write_role(Role::SysOp)
            .with_sort_order(100);

        let board = repo.create(&new_board).unwrap();

        assert_eq!(board.name, "admin-board");
        assert_eq!(
            board.description,
            Some("For administrators only".to_string())
        );
        assert_eq!(board.board_type, BoardType::Flat);
        assert_eq!(board.min_read_role, Role::SubOp);
        assert_eq!(board.min_write_role, Role::SysOp);
        assert_eq!(board.sort_order, 100);
    }

    #[test]
    fn test_create_duplicate_name() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general");
        repo.create(&new_board).unwrap();

        let duplicate = NewBoard::new("general");
        let result = repo.create(&duplicate);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_by_id() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general");
        let created = repo.create(&new_board).unwrap();

        let found = repo.get_by_id(created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "general");

        let not_found = repo.get_by_id(999).unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_by_name() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general").with_description("General discussion");
        repo.create(&new_board).unwrap();

        let found = repo.get_by_name("general").unwrap();
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().description,
            Some("General discussion".to_string())
        );

        let not_found = repo.get_by_name("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_update_board() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).unwrap();

        let update = BoardUpdate::new()
            .name("renamed")
            .description(Some("Updated description".to_string()))
            .board_type(BoardType::Flat)
            .min_read_role(Role::Member);

        let updated = repo.update(board.id, &update).unwrap().unwrap();

        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.description, Some("Updated description".to_string()));
        assert_eq!(updated.board_type, BoardType::Flat);
        assert_eq!(updated.min_read_role, Role::Member);
        // Unchanged fields
        assert_eq!(updated.min_write_role, Role::Member);
    }

    #[test]
    fn test_update_nonexistent_board() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let update = BoardUpdate::new().name("New Name");
        let result = repo.update(999, &update).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_update_empty() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).unwrap();

        let update = BoardUpdate::new();
        let result = repo.update(board.id, &update).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "general");
    }

    #[test]
    fn test_update_is_active() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).unwrap();
        assert!(board.is_active);

        let update = BoardUpdate::new().is_active(false);
        let updated = repo.update(board.id, &update).unwrap().unwrap();

        assert!(!updated.is_active);
    }

    #[test]
    fn test_update_clear_description() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general").with_description("Has description");
        let board = repo.create(&new_board).unwrap();
        assert!(board.description.is_some());

        let update = BoardUpdate::new().description(None);
        let updated = repo.update(board.id, &update).unwrap().unwrap();

        assert!(updated.description.is_none());
    }

    #[test]
    fn test_delete_board() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).unwrap();

        let deleted = repo.delete(board.id).unwrap();
        assert!(deleted);

        let found = repo.get_by_id(board.id).unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(board.id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_list_active() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        // Create some boards with different sort orders
        repo.create(&NewBoard::new("board3").with_sort_order(30))
            .unwrap();
        let board2 = repo
            .create(&NewBoard::new("board2").with_sort_order(20))
            .unwrap();
        repo.create(&NewBoard::new("board1").with_sort_order(10))
            .unwrap();

        // Deactivate board2
        repo.update(board2.id, &BoardUpdate::new().is_active(false))
            .unwrap();

        let active = repo.list_active().unwrap();
        assert_eq!(active.len(), 2);
        // Should be sorted by sort_order
        assert_eq!(active[0].name, "board1");
        assert_eq!(active[1].name, "board3");
    }

    #[test]
    fn test_list_all() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        repo.create(&NewBoard::new("board1").with_sort_order(10))
            .unwrap();
        let board2 = repo
            .create(&NewBoard::new("board2").with_sort_order(20))
            .unwrap();
        repo.create(&NewBoard::new("board3").with_sort_order(30))
            .unwrap();

        // Deactivate board2
        repo.update(board2.id, &BoardUpdate::new().is_active(false))
            .unwrap();

        let all = repo.list_all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_accessible() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        // Create boards with different read permissions
        repo.create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .unwrap();
        repo.create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .unwrap();
        repo.create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .unwrap();
        repo.create(&NewBoard::new("admin").with_min_read_role(Role::SysOp))
            .unwrap();

        // Guest can only see public
        let guest_boards = repo.list_accessible(Role::Guest).unwrap();
        assert_eq!(guest_boards.len(), 1);
        assert_eq!(guest_boards[0].name, "public");

        // Member can see public and members
        let member_boards = repo.list_accessible(Role::Member).unwrap();
        assert_eq!(member_boards.len(), 2);

        // SubOp can see public, members, staff
        let subop_boards = repo.list_accessible(Role::SubOp).unwrap();
        assert_eq!(subop_boards.len(), 3);

        // SysOp can see all
        let sysop_boards = repo.list_accessible(Role::SysOp).unwrap();
        assert_eq!(sysop_boards.len(), 4);
    }

    #[test]
    fn test_list_writable() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        // Create boards with different write permissions
        repo.create(&NewBoard::new("public").with_min_write_role(Role::Guest))
            .unwrap();
        repo.create(&NewBoard::new("members").with_min_write_role(Role::Member))
            .unwrap();
        repo.create(&NewBoard::new("staff").with_min_write_role(Role::SubOp))
            .unwrap();

        // Guest can only write to public
        let guest_boards = repo.list_writable(Role::Guest).unwrap();
        assert_eq!(guest_boards.len(), 1);

        // Member can write to public and members
        let member_boards = repo.list_writable(Role::Member).unwrap();
        assert_eq!(member_boards.len(), 2);

        // SubOp can write to all
        let subop_boards = repo.list_writable(Role::SubOp).unwrap();
        assert_eq!(subop_boards.len(), 3);
    }

    #[test]
    fn test_count() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        assert_eq!(repo.count().unwrap(), 0);
        assert_eq!(repo.count_active().unwrap(), 0);

        repo.create(&NewBoard::new("board1")).unwrap();
        let board2 = repo.create(&NewBoard::new("board2")).unwrap();

        assert_eq!(repo.count().unwrap(), 2);
        assert_eq!(repo.count_active().unwrap(), 2);

        repo.update(board2.id, &BoardUpdate::new().is_active(false))
            .unwrap();

        assert_eq!(repo.count().unwrap(), 2);
        assert_eq!(repo.count_active().unwrap(), 1);
    }

    #[test]
    fn test_name_exists() {
        let db = setup_db();
        let repo = BoardRepository::new(&db);

        assert!(!repo.name_exists("general").unwrap());

        repo.create(&NewBoard::new("general")).unwrap();

        assert!(repo.name_exists("general").unwrap());
        assert!(!repo.name_exists("other").unwrap());
    }
}
