//! Board repository for HOBBS.
//!
//! This module provides CRUD operations for boards in the database.

use sqlx::QueryBuilder;

use super::types::{Board, BoardType, BoardUpdate, NewBoard};
use crate::db::{DbPool, Role, SQL_TRUE};
use crate::{HobbsError, Result};

/// Repository for board CRUD operations.
pub struct BoardRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> BoardRepository<'a> {
    /// Create a new BoardRepository with the given database pool reference.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new board in the database.
    ///
    /// Returns the created board with the assigned ID.
    #[cfg(feature = "sqlite")]
    pub async fn create(&self, new_board: &NewBoard) -> Result<Board> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO boards (name, description, board_type, min_read_role, min_write_role, sort_order)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&new_board.name)
        .bind(&new_board.description)
        .bind(new_board.board_type.as_str())
        .bind(new_board.min_read_role.as_str())
        .bind(new_board.min_write_role.as_str())
        .bind(new_board.sort_order)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("board".to_string()))
    }

    /// Create a new board in the database.
    ///
    /// Returns the created board with the assigned ID.
    #[cfg(feature = "postgres")]
    pub async fn create(&self, new_board: &NewBoard) -> Result<Board> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO boards (name, description, board_type, min_read_role, min_write_role, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(&new_board.name)
        .bind(&new_board.description)
        .bind(new_board.board_type.as_str())
        .bind(new_board.min_read_role.as_str())
        .bind(new_board.min_write_role.as_str())
        .bind(new_board.sort_order)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("board".to_string()))
    }

    /// Get a board by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Board>> {
        let result: Option<BoardRow> = sqlx::query_as(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(|row| row.into_board()))
    }

    /// Get a board by name.
    pub async fn get_by_name(&self, name: &str) -> Result<Option<Board>> {
        let result: Option<BoardRow> = sqlx::query_as(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(|row| row.into_board()))
    }

    /// Update a board by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated board, or None if not found.
    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: i64, update: &BoardUpdate) -> Result<Option<Board>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        let mut query: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("UPDATE boards SET ");
        let mut separated = query.separated(", ");

        if let Some(ref name) = update.name {
            separated.push("name = ");
            separated.push_bind_unseparated(name);
        }
        if let Some(ref description) = update.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description.clone());
        }
        if let Some(board_type) = update.board_type {
            separated.push("board_type = ");
            separated.push_bind_unseparated(board_type.as_str().to_string());
        }
        if let Some(role) = update.min_read_role {
            separated.push("min_read_role = ");
            separated.push_bind_unseparated(role.as_str().to_string());
        }
        if let Some(role) = update.min_write_role {
            separated.push("min_write_role = ");
            separated.push_bind_unseparated(role.as_str().to_string());
        }
        if let Some(sort_order) = update.sort_order {
            separated.push("sort_order = ");
            separated.push_bind_unseparated(sort_order);
        }
        if let Some(is_active) = update.is_active {
            separated.push("is_active = ");
            separated.push_bind_unseparated(is_active);
        }

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_by_id(id).await
    }

    /// Update a board by ID.
    ///
    /// Only fields that are set in the update will be modified.
    /// Returns the updated board, or None if not found.
    #[cfg(feature = "postgres")]
    pub async fn update(&self, id: i64, update: &BoardUpdate) -> Result<Option<Board>> {
        if update.is_empty() {
            return self.get_by_id(id).await;
        }

        let mut query: QueryBuilder<sqlx::Postgres> = QueryBuilder::new("UPDATE boards SET ");
        let mut separated = query.separated(", ");

        if let Some(ref name) = update.name {
            separated.push("name = ");
            separated.push_bind_unseparated(name);
        }
        if let Some(ref description) = update.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description.clone());
        }
        if let Some(board_type) = update.board_type {
            separated.push("board_type = ");
            separated.push_bind_unseparated(board_type.as_str().to_string());
        }
        if let Some(role) = update.min_read_role {
            separated.push("min_read_role = ");
            separated.push_bind_unseparated(role.as_str().to_string());
        }
        if let Some(role) = update.min_write_role {
            separated.push("min_write_role = ");
            separated.push_bind_unseparated(role.as_str().to_string());
        }
        if let Some(sort_order) = update.sort_order {
            separated.push("sort_order = ");
            separated.push_bind_unseparated(sort_order);
        }
        if let Some(is_active) = update.is_active {
            separated.push("is_active = ");
            separated.push_bind_unseparated(is_active);
        }

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_by_id(id).await
    }

    /// Delete a board by ID.
    ///
    /// Returns true if a board was deleted, false if not found.
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM boards WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }

    /// List all active boards, ordered by sort_order then created_at.
    pub async fn list_active(&self) -> Result<Vec<Board>> {
        let query = format!(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards WHERE is_active = {} ORDER BY sort_order ASC, created_at ASC, id ASC",
            SQL_TRUE
        );
        let rows: Vec<BoardRow> = sqlx::query_as(&query)
            .fetch_all(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|row| row.into_board()).collect())
    }

    /// List all boards (including inactive), ordered by sort_order then created_at.
    pub async fn list_all(&self) -> Result<Vec<Board>> {
        let rows: Vec<BoardRow> = sqlx::query_as(
            "SELECT id, name, description, board_type, min_read_role, min_write_role,
                    sort_order, is_active, created_at
             FROM boards ORDER BY sort_order ASC, created_at ASC, id ASC",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|row| row.into_board()).collect())
    }

    /// List boards accessible by a user with the given role.
    ///
    /// Only returns active boards where min_read_role <= user's role.
    pub async fn list_accessible(&self, user_role: Role) -> Result<Vec<Board>> {
        let all_boards = self.list_active().await?;
        let accessible = all_boards
            .into_iter()
            .filter(|b| b.can_read(user_role))
            .collect();
        Ok(accessible)
    }

    /// List boards writable by a user with the given role.
    ///
    /// Only returns active boards where min_write_role <= user's role.
    pub async fn list_writable(&self, user_role: Role) -> Result<Vec<Board>> {
        let all_boards = self.list_active().await?;
        let writable = all_boards
            .into_iter()
            .filter(|b| b.can_write(user_role))
            .collect();
        Ok(writable)
    }

    /// Count all boards.
    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM boards")
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(count.0)
    }

    /// Count active boards.
    pub async fn count_active(&self) -> Result<i64> {
        let query = format!("SELECT COUNT(*) FROM boards WHERE is_active = {}", SQL_TRUE);
        let count: (i64,) = sqlx::query_as(&query)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(count.0)
    }

    /// Check if a board name is already taken.
    pub async fn name_exists(&self, name: &str) -> Result<bool> {
        let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM boards WHERE name = $1)")
            .bind(name)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;
        Ok(exists.0)
    }
}

/// Internal struct for mapping database rows to Board.
#[derive(sqlx::FromRow)]
struct BoardRow {
    id: i64,
    name: String,
    description: Option<String>,
    board_type: String,
    min_read_role: String,
    min_write_role: String,
    sort_order: i32,
    is_active: bool,
    created_at: String,
}

impl BoardRow {
    fn into_board(self) -> Board {
        Board {
            id: self.id,
            name: self.name,
            description: self.description,
            board_type: self.board_type.parse().unwrap_or(BoardType::Thread),
            min_read_role: self.min_read_role.parse().unwrap_or(Role::Guest),
            min_write_role: self.min_write_role.parse().unwrap_or(Role::Member),
            sort_order: self.sort_order,
            is_active: self.is_active,
            created_at: self.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_create_board() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).await.unwrap();

        assert_eq!(board.id, 1);
        assert_eq!(board.name, "general");
        assert_eq!(board.board_type, BoardType::Thread);
        assert_eq!(board.min_read_role, Role::Guest);
        assert_eq!(board.min_write_role, Role::Member);
        assert!(board.is_active);
    }

    #[tokio::test]
    async fn test_create_board_with_options() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("admin-board")
            .with_description("For administrators only")
            .with_board_type(BoardType::Flat)
            .with_min_read_role(Role::SubOp)
            .with_min_write_role(Role::SysOp)
            .with_sort_order(100);

        let board = repo.create(&new_board).await.unwrap();

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

    #[tokio::test]
    async fn test_create_duplicate_name() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general");
        repo.create(&new_board).await.unwrap();

        let duplicate = NewBoard::new("general");
        let result = repo.create(&duplicate).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general");
        let created = repo.create(&new_board).await.unwrap();

        let found = repo.get_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "general");

        let not_found = repo.get_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general").with_description("General discussion");
        repo.create(&new_board).await.unwrap();

        let found = repo.get_by_name("general").await.unwrap();
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().description,
            Some("General discussion".to_string())
        );

        let not_found = repo.get_by_name("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_board() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).await.unwrap();

        let update = BoardUpdate::new()
            .name("renamed")
            .description(Some("Updated description".to_string()))
            .board_type(BoardType::Flat)
            .min_read_role(Role::Member);

        let updated = repo.update(board.id, &update).await.unwrap().unwrap();

        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.description, Some("Updated description".to_string()));
        assert_eq!(updated.board_type, BoardType::Flat);
        assert_eq!(updated.min_read_role, Role::Member);
        // Unchanged fields
        assert_eq!(updated.min_write_role, Role::Member);
    }

    #[tokio::test]
    async fn test_update_nonexistent_board() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let update = BoardUpdate::new().name("New Name");
        let result = repo.update(999, &update).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_empty() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).await.unwrap();

        let update = BoardUpdate::new();
        let result = repo.update(board.id, &update).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "general");
    }

    #[tokio::test]
    async fn test_update_is_active() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).await.unwrap();
        assert!(board.is_active);

        let update = BoardUpdate::new().is_active(false);
        let updated = repo.update(board.id, &update).await.unwrap().unwrap();

        assert!(!updated.is_active);
    }

    #[tokio::test]
    async fn test_update_clear_description() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general").with_description("Has description");
        let board = repo.create(&new_board).await.unwrap();
        assert!(board.description.is_some());

        let update = BoardUpdate::new().description(None);
        let updated = repo.update(board.id, &update).await.unwrap().unwrap();

        assert!(updated.description.is_none());
    }

    #[tokio::test]
    async fn test_delete_board() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        let new_board = NewBoard::new("general");
        let board = repo.create(&new_board).await.unwrap();

        let deleted = repo.delete(board.id).await.unwrap();
        assert!(deleted);

        let found = repo.get_by_id(board.id).await.unwrap();
        assert!(found.is_none());

        // Deleting again should return false
        let deleted_again = repo.delete(board.id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_list_active() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        // Create some boards with different sort orders
        repo.create(&NewBoard::new("board3").with_sort_order(30))
            .await
            .unwrap();
        let board2 = repo
            .create(&NewBoard::new("board2").with_sort_order(20))
            .await
            .unwrap();
        repo.create(&NewBoard::new("board1").with_sort_order(10))
            .await
            .unwrap();

        // Deactivate board2
        repo.update(board2.id, &BoardUpdate::new().is_active(false))
            .await
            .unwrap();

        let active = repo.list_active().await.unwrap();
        assert_eq!(active.len(), 2);
        // Should be sorted by sort_order
        assert_eq!(active[0].name, "board1");
        assert_eq!(active[1].name, "board3");
    }

    #[tokio::test]
    async fn test_list_all() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        repo.create(&NewBoard::new("board1").with_sort_order(10))
            .await
            .unwrap();
        let board2 = repo
            .create(&NewBoard::new("board2").with_sort_order(20))
            .await
            .unwrap();
        repo.create(&NewBoard::new("board3").with_sort_order(30))
            .await
            .unwrap();

        // Deactivate board2
        repo.update(board2.id, &BoardUpdate::new().is_active(false))
            .await
            .unwrap();

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_list_accessible() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        // Create boards with different read permissions
        repo.create(&NewBoard::new("public").with_min_read_role(Role::Guest))
            .await
            .unwrap();
        repo.create(&NewBoard::new("members").with_min_read_role(Role::Member))
            .await
            .unwrap();
        repo.create(&NewBoard::new("staff").with_min_read_role(Role::SubOp))
            .await
            .unwrap();
        repo.create(&NewBoard::new("admin").with_min_read_role(Role::SysOp))
            .await
            .unwrap();

        // Guest can only see public
        let guest_boards = repo.list_accessible(Role::Guest).await.unwrap();
        assert_eq!(guest_boards.len(), 1);
        assert_eq!(guest_boards[0].name, "public");

        // Member can see public and members
        let member_boards = repo.list_accessible(Role::Member).await.unwrap();
        assert_eq!(member_boards.len(), 2);

        // SubOp can see public, members, staff
        let subop_boards = repo.list_accessible(Role::SubOp).await.unwrap();
        assert_eq!(subop_boards.len(), 3);

        // SysOp can see all
        let sysop_boards = repo.list_accessible(Role::SysOp).await.unwrap();
        assert_eq!(sysop_boards.len(), 4);
    }

    #[tokio::test]
    async fn test_list_writable() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        // Create boards with different write permissions
        repo.create(&NewBoard::new("public").with_min_write_role(Role::Guest))
            .await
            .unwrap();
        repo.create(&NewBoard::new("members").with_min_write_role(Role::Member))
            .await
            .unwrap();
        repo.create(&NewBoard::new("staff").with_min_write_role(Role::SubOp))
            .await
            .unwrap();

        // Guest can only write to public
        let guest_boards = repo.list_writable(Role::Guest).await.unwrap();
        assert_eq!(guest_boards.len(), 1);

        // Member can write to public and members
        let member_boards = repo.list_writable(Role::Member).await.unwrap();
        assert_eq!(member_boards.len(), 2);

        // SubOp can write to all
        let subop_boards = repo.list_writable(Role::SubOp).await.unwrap();
        assert_eq!(subop_boards.len(), 3);
    }

    #[tokio::test]
    async fn test_count() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        assert_eq!(repo.count().await.unwrap(), 0);
        assert_eq!(repo.count_active().await.unwrap(), 0);

        repo.create(&NewBoard::new("board1")).await.unwrap();
        let board2 = repo.create(&NewBoard::new("board2")).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_active().await.unwrap(), 2);

        repo.update(board2.id, &BoardUpdate::new().is_active(false))
            .await
            .unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_active().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_name_exists() {
        let db = setup_db().await;
        let repo = BoardRepository::new(db.pool());

        assert!(!repo.name_exists("general").await.unwrap());

        repo.create(&NewBoard::new("general")).await.unwrap();

        assert!(repo.name_exists("general").await.unwrap());
        assert!(!repo.name_exists("other").await.unwrap());
    }
}
