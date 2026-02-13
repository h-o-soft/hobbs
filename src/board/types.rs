//! Board model for HOBBS.
//!
//! This module defines the Board struct and BoardType enum for bulletin board management.

use std::fmt;
use std::str::FromStr;

use crate::db::Role;

/// Board type for distinguishing between thread-based and flat boards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoardType {
    /// Thread-based board where posts are grouped into threads.
    #[default]
    Thread,
    /// Flat board where posts are displayed in chronological order.
    Flat,
}

impl BoardType {
    /// Convert board type to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            BoardType::Thread => "thread",
            BoardType::Flat => "flat",
        }
    }

    /// Get display name for the board type.
    pub fn display_name(&self) -> &'static str {
        match self {
            BoardType::Thread => "スレッド形式",
            BoardType::Flat => "フラット形式",
        }
    }
}

impl fmt::Display for BoardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BoardType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "thread" => Ok(BoardType::Thread),
            "flat" => Ok(BoardType::Flat),
            _ => Err(format!("unknown board type: {s}")),
        }
    }
}

/// Board entity representing a bulletin board.
#[derive(Debug, Clone)]
pub struct Board {
    /// Unique board ID.
    pub id: i64,
    /// Board name (unique).
    pub name: String,
    /// Board description.
    pub description: Option<String>,
    /// Board type (thread or flat).
    pub board_type: BoardType,
    /// Minimum role required to read posts.
    pub min_read_role: Role,
    /// Minimum role required to write posts.
    pub min_write_role: Role,
    /// Sort order for display.
    pub sort_order: i32,
    /// Whether the board is active.
    pub is_active: bool,
    /// Whether auto-paging is disabled for this board.
    pub disable_paging: bool,
    /// Board creation timestamp.
    pub created_at: String,
}

impl Board {
    /// Check if a user with the given role can read this board.
    pub fn can_read(&self, role: Role) -> bool {
        role.can_access(self.min_read_role)
    }

    /// Check if a user with the given role can write to this board.
    pub fn can_write(&self, role: Role) -> bool {
        role.can_access(self.min_write_role)
    }
}

/// Data for creating a new board.
#[derive(Debug, Clone)]
pub struct NewBoard {
    /// Board name.
    pub name: String,
    /// Board description.
    pub description: Option<String>,
    /// Board type (defaults to Thread).
    pub board_type: BoardType,
    /// Minimum role required to read posts (defaults to Guest).
    pub min_read_role: Role,
    /// Minimum role required to write posts (defaults to Member).
    pub min_write_role: Role,
    /// Sort order for display (defaults to 0).
    pub sort_order: i32,
    /// Whether auto-paging is disabled for this board.
    pub disable_paging: bool,
}

impl NewBoard {
    /// Create a new board with minimal required fields.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            board_type: BoardType::Thread,
            min_read_role: Role::Guest,
            min_write_role: Role::Member,
            sort_order: 0,
            disable_paging: false,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the board type.
    pub fn with_board_type(mut self, board_type: BoardType) -> Self {
        self.board_type = board_type;
        self
    }

    /// Set the minimum read role.
    pub fn with_min_read_role(mut self, role: Role) -> Self {
        self.min_read_role = role;
        self
    }

    /// Set the minimum write role.
    pub fn with_min_write_role(mut self, role: Role) -> Self {
        self.min_write_role = role;
        self
    }

    /// Set the sort order.
    pub fn with_sort_order(mut self, sort_order: i32) -> Self {
        self.sort_order = sort_order;
        self
    }

    /// Set whether auto-paging is disabled.
    pub fn with_disable_paging(mut self, disable_paging: bool) -> Self {
        self.disable_paging = disable_paging;
        self
    }
}

/// Data for updating an existing board.
#[derive(Debug, Clone, Default)]
pub struct BoardUpdate {
    /// New name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<Option<String>>,
    /// New board type.
    pub board_type: Option<BoardType>,
    /// New minimum read role.
    pub min_read_role: Option<Role>,
    /// New minimum write role.
    pub min_write_role: Option<Role>,
    /// New sort order.
    pub sort_order: Option<i32>,
    /// New active status.
    pub is_active: Option<bool>,
    /// New disable_paging status.
    pub disable_paging: Option<bool>,
}

impl BoardUpdate {
    /// Create an empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set new name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set new description.
    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = Some(description);
        self
    }

    /// Set new board type.
    pub fn board_type(mut self, board_type: BoardType) -> Self {
        self.board_type = Some(board_type);
        self
    }

    /// Set new minimum read role.
    pub fn min_read_role(mut self, role: Role) -> Self {
        self.min_read_role = Some(role);
        self
    }

    /// Set new minimum write role.
    pub fn min_write_role(mut self, role: Role) -> Self {
        self.min_write_role = Some(role);
        self
    }

    /// Set new sort order.
    pub fn sort_order(mut self, sort_order: i32) -> Self {
        self.sort_order = Some(sort_order);
        self
    }

    /// Set active status.
    pub fn is_active(mut self, is_active: bool) -> Self {
        self.is_active = Some(is_active);
        self
    }

    /// Set disable_paging status.
    pub fn disable_paging(mut self, disable_paging: bool) -> Self {
        self.disable_paging = Some(disable_paging);
        self
    }

    /// Check if any fields are set.
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.description.is_none()
            && self.board_type.is_none()
            && self.min_read_role.is_none()
            && self.min_write_role.is_none()
            && self.sort_order.is_none()
            && self.is_active.is_none()
            && self.disable_paging.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_type_as_str() {
        assert_eq!(BoardType::Thread.as_str(), "thread");
        assert_eq!(BoardType::Flat.as_str(), "flat");
    }

    #[test]
    fn test_board_type_display_name() {
        assert_eq!(BoardType::Thread.display_name(), "スレッド形式");
        assert_eq!(BoardType::Flat.display_name(), "フラット形式");
    }

    #[test]
    fn test_board_type_from_str() {
        assert_eq!(BoardType::from_str("thread").unwrap(), BoardType::Thread);
        assert_eq!(BoardType::from_str("flat").unwrap(), BoardType::Flat);
        assert_eq!(BoardType::from_str("THREAD").unwrap(), BoardType::Thread);
        assert!(BoardType::from_str("invalid").is_err());
    }

    #[test]
    fn test_board_type_display() {
        assert_eq!(format!("{}", BoardType::Thread), "thread");
        assert_eq!(format!("{}", BoardType::Flat), "flat");
    }

    #[test]
    fn test_board_type_default() {
        assert_eq!(BoardType::default(), BoardType::Thread);
    }

    #[test]
    fn test_board_can_read() {
        let board = Board {
            id: 1,
            name: "test".to_string(),
            description: None,
            board_type: BoardType::Thread,
            min_read_role: Role::Member,
            min_write_role: Role::Member,
            sort_order: 0,
            is_active: true,
            disable_paging: false,
            created_at: "2024-01-01".to_string(),
        };

        assert!(!board.can_read(Role::Guest));
        assert!(board.can_read(Role::Member));
        assert!(board.can_read(Role::SubOp));
        assert!(board.can_read(Role::SysOp));
    }

    #[test]
    fn test_board_can_write() {
        let board = Board {
            id: 1,
            name: "test".to_string(),
            description: None,
            board_type: BoardType::Thread,
            min_read_role: Role::Guest,
            min_write_role: Role::SubOp,
            sort_order: 0,
            is_active: true,
            disable_paging: false,
            created_at: "2024-01-01".to_string(),
        };

        assert!(!board.can_write(Role::Guest));
        assert!(!board.can_write(Role::Member));
        assert!(board.can_write(Role::SubOp));
        assert!(board.can_write(Role::SysOp));
    }

    #[test]
    fn test_new_board_builder() {
        let board = NewBoard::new("Test Board")
            .with_description("Test description")
            .with_board_type(BoardType::Flat)
            .with_min_read_role(Role::Member)
            .with_min_write_role(Role::SubOp)
            .with_sort_order(10);

        assert_eq!(board.name, "Test Board");
        assert_eq!(board.description, Some("Test description".to_string()));
        assert_eq!(board.board_type, BoardType::Flat);
        assert_eq!(board.min_read_role, Role::Member);
        assert_eq!(board.min_write_role, Role::SubOp);
        assert_eq!(board.sort_order, 10);
    }

    #[test]
    fn test_new_board_defaults() {
        let board = NewBoard::new("Test Board");

        assert_eq!(board.name, "Test Board");
        assert_eq!(board.description, None);
        assert_eq!(board.board_type, BoardType::Thread);
        assert_eq!(board.min_read_role, Role::Guest);
        assert_eq!(board.min_write_role, Role::Member);
        assert_eq!(board.sort_order, 0);
    }

    #[test]
    fn test_board_update_builder() {
        let update = BoardUpdate::new()
            .name("New Name")
            .description(Some("New description".to_string()))
            .board_type(BoardType::Flat)
            .min_read_role(Role::Member)
            .is_active(false);

        assert_eq!(update.name, Some("New Name".to_string()));
        assert_eq!(
            update.description,
            Some(Some("New description".to_string()))
        );
        assert_eq!(update.board_type, Some(BoardType::Flat));
        assert_eq!(update.min_read_role, Some(Role::Member));
        assert_eq!(update.is_active, Some(false));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_board_update_empty() {
        let update = BoardUpdate::new();
        assert!(update.is_empty());
    }

    #[test]
    fn test_board_update_clear_description() {
        let update = BoardUpdate::new().description(None);
        assert_eq!(update.description, Some(None));
        assert!(!update.is_empty());
    }
}
