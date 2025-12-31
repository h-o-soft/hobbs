//! Chat command parser and handlers for HOBBS.
//!
//! This module provides parsing and handling of chat commands like
//! /quit, /who, /me, and /help.

/// Result of parsing a chat input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatInput {
    /// Regular chat message.
    Message(String),
    /// Parsed command.
    Command(ChatCommand),
}

/// A parsed chat command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatCommand {
    /// Exit the chat room.
    Quit,
    /// List participants in the room.
    Who,
    /// Send an action message (e.g., "/me yawns" -> "* user yawns").
    Me(String),
    /// Show help message.
    Help,
    /// Unknown command.
    Unknown(String),
}

impl ChatCommand {
    /// Get the command name.
    pub fn name(&self) -> &str {
        match self {
            ChatCommand::Quit => "quit",
            ChatCommand::Who => "who",
            ChatCommand::Me(_) => "me",
            ChatCommand::Help => "help",
            ChatCommand::Unknown(cmd) => cmd,
        }
    }
}

impl std::fmt::Display for ChatCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatCommand::Quit => write!(f, "/quit"),
            ChatCommand::Who => write!(f, "/who"),
            ChatCommand::Me(action) => write!(f, "/me {action}"),
            ChatCommand::Help => write!(f, "/help"),
            ChatCommand::Unknown(cmd) => write!(f, "/{cmd}"),
        }
    }
}

/// Parse a chat input line into a message or command.
pub fn parse_input(input: &str) -> ChatInput {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return ChatInput::Message(String::new());
    }

    if !trimmed.starts_with('/') {
        return ChatInput::Message(trimmed.to_string());
    }

    // Parse command
    let without_slash = &trimmed[1..];
    let (cmd, args) = match without_slash.find(' ') {
        Some(pos) => (&without_slash[..pos], without_slash[pos + 1..].trim()),
        None => (without_slash, ""),
    };

    let command = match cmd.to_lowercase().as_str() {
        "quit" | "q" | "exit" => ChatCommand::Quit,
        "who" | "w" | "users" | "list" => ChatCommand::Who,
        "me" | "action" => {
            if args.is_empty() {
                ChatCommand::Me(String::new())
            } else {
                ChatCommand::Me(args.to_string())
            }
        }
        "help" | "h" | "?" => ChatCommand::Help,
        _ => ChatCommand::Unknown(cmd.to_string()),
    };

    ChatInput::Command(command)
}

/// Chat command information for help display.
pub struct CommandInfo {
    /// Command name.
    pub name: &'static str,
    /// Command aliases.
    pub aliases: &'static [&'static str],
    /// Command syntax.
    pub syntax: &'static str,
    /// Command description.
    pub description: &'static str,
}

/// Get all available command information.
pub fn get_command_help() -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            name: "quit",
            aliases: &["q", "exit"],
            syntax: "/quit",
            description: "チャットルームを退室します",
        },
        CommandInfo {
            name: "who",
            aliases: &["w", "users", "list"],
            syntax: "/who",
            description: "参加者一覧を表示します",
        },
        CommandInfo {
            name: "me",
            aliases: &["action"],
            syntax: "/me <アクション>",
            description: "アクションメッセージを送信します (例: /me yawns → * user yawns)",
        },
        CommandInfo {
            name: "help",
            aliases: &["h", "?"],
            syntax: "/help",
            description: "コマンドヘルプを表示します",
        },
    ]
}

/// Format the help message for display.
pub fn format_help() -> String {
    let mut lines = Vec::new();
    lines.push("=== チャットコマンド ===".to_string());
    lines.push(String::new());

    for info in get_command_help() {
        lines.push(info.syntax.to_string());
        if !info.aliases.is_empty() {
            lines.push(format!("  別名: /{}", info.aliases.join(", /")));
        }
        lines.push(format!("  {}", info.description));
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Format the participant list for display.
pub fn format_who(participants: &[String], room_name: &str) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "=== {} の参加者 ({}) ===",
        room_name,
        participants.len()
    ));

    if participants.is_empty() {
        lines.push("(参加者はいません)".to_string());
    } else {
        for name in participants {
            lines.push(format!("  {name}"));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_regular_message() {
        let input = parse_input("Hello, world!");
        assert_eq!(input, ChatInput::Message("Hello, world!".to_string()));
    }

    #[test]
    fn test_parse_message_with_leading_whitespace() {
        let input = parse_input("  Hello!");
        assert_eq!(input, ChatInput::Message("Hello!".to_string()));
    }

    #[test]
    fn test_parse_empty_message() {
        let input = parse_input("");
        assert_eq!(input, ChatInput::Message(String::new()));
    }

    #[test]
    fn test_parse_whitespace_only() {
        let input = parse_input("   ");
        assert_eq!(input, ChatInput::Message(String::new()));
    }

    #[test]
    fn test_parse_quit_command() {
        assert_eq!(parse_input("/quit"), ChatInput::Command(ChatCommand::Quit));
        assert_eq!(parse_input("/q"), ChatInput::Command(ChatCommand::Quit));
        assert_eq!(parse_input("/exit"), ChatInput::Command(ChatCommand::Quit));
    }

    #[test]
    fn test_parse_quit_case_insensitive() {
        assert_eq!(parse_input("/QUIT"), ChatInput::Command(ChatCommand::Quit));
        assert_eq!(parse_input("/Quit"), ChatInput::Command(ChatCommand::Quit));
    }

    #[test]
    fn test_parse_who_command() {
        assert_eq!(parse_input("/who"), ChatInput::Command(ChatCommand::Who));
        assert_eq!(parse_input("/w"), ChatInput::Command(ChatCommand::Who));
        assert_eq!(parse_input("/users"), ChatInput::Command(ChatCommand::Who));
        assert_eq!(parse_input("/list"), ChatInput::Command(ChatCommand::Who));
    }

    #[test]
    fn test_parse_me_command() {
        assert_eq!(
            parse_input("/me yawns"),
            ChatInput::Command(ChatCommand::Me("yawns".to_string()))
        );
        assert_eq!(
            parse_input("/action waves"),
            ChatInput::Command(ChatCommand::Me("waves".to_string()))
        );
    }

    #[test]
    fn test_parse_me_with_multiple_words() {
        assert_eq!(
            parse_input("/me waves at everyone"),
            ChatInput::Command(ChatCommand::Me("waves at everyone".to_string()))
        );
    }

    #[test]
    fn test_parse_me_empty() {
        assert_eq!(
            parse_input("/me"),
            ChatInput::Command(ChatCommand::Me(String::new()))
        );
        assert_eq!(
            parse_input("/me "),
            ChatInput::Command(ChatCommand::Me(String::new()))
        );
    }

    #[test]
    fn test_parse_help_command() {
        assert_eq!(parse_input("/help"), ChatInput::Command(ChatCommand::Help));
        assert_eq!(parse_input("/h"), ChatInput::Command(ChatCommand::Help));
        assert_eq!(parse_input("/?"), ChatInput::Command(ChatCommand::Help));
    }

    #[test]
    fn test_parse_unknown_command() {
        assert_eq!(
            parse_input("/unknown"),
            ChatInput::Command(ChatCommand::Unknown("unknown".to_string()))
        );
        assert_eq!(
            parse_input("/foo bar"),
            ChatInput::Command(ChatCommand::Unknown("foo".to_string()))
        );
    }

    #[test]
    fn test_parse_command_with_leading_whitespace() {
        assert_eq!(
            parse_input("  /quit"),
            ChatInput::Command(ChatCommand::Quit)
        );
    }

    #[test]
    fn test_chat_command_name() {
        assert_eq!(ChatCommand::Quit.name(), "quit");
        assert_eq!(ChatCommand::Who.name(), "who");
        assert_eq!(ChatCommand::Me("test".to_string()).name(), "me");
        assert_eq!(ChatCommand::Help.name(), "help");
        assert_eq!(ChatCommand::Unknown("foo".to_string()).name(), "foo");
    }

    #[test]
    fn test_chat_command_display() {
        assert_eq!(format!("{}", ChatCommand::Quit), "/quit");
        assert_eq!(format!("{}", ChatCommand::Who), "/who");
        assert_eq!(
            format!("{}", ChatCommand::Me("waves".to_string())),
            "/me waves"
        );
        assert_eq!(format!("{}", ChatCommand::Help), "/help");
        assert_eq!(
            format!("{}", ChatCommand::Unknown("foo".to_string())),
            "/foo"
        );
    }

    #[test]
    fn test_get_command_help() {
        let help = get_command_help();
        assert_eq!(help.len(), 4);

        let quit_info = &help[0];
        assert_eq!(quit_info.name, "quit");
        assert!(quit_info.aliases.contains(&"q"));
        assert!(quit_info.aliases.contains(&"exit"));
    }

    #[test]
    fn test_format_help() {
        let help = format_help();
        assert!(help.contains("チャットコマンド"));
        assert!(help.contains("/quit"));
        assert!(help.contains("/who"));
        assert!(help.contains("/me"));
        assert!(help.contains("/help"));
    }

    #[test]
    fn test_format_who_with_participants() {
        let participants = vec!["Alice".to_string(), "Bob".to_string()];
        let result = format_who(&participants, "Lobby");

        assert!(result.contains("Lobby"));
        assert!(result.contains("(2)"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_format_who_empty() {
        let participants: Vec<String> = vec![];
        let result = format_who(&participants, "Empty Room");

        assert!(result.contains("Empty Room"));
        assert!(result.contains("(0)"));
        assert!(result.contains("参加者はいません"));
    }

    #[test]
    fn test_message_starting_with_slash_space() {
        // Messages like "/ something" should be treated as messages, not commands
        // Actually, based on our logic, "/ " starts with '/' so it will try to parse
        // Let's verify the behavior
        let input = parse_input("/ test");
        // This parses as an empty command name, which becomes Unknown("")
        assert!(matches!(input, ChatInput::Command(ChatCommand::Unknown(_))));
    }

    #[test]
    fn test_slash_only() {
        let input = parse_input("/");
        // Empty command name becomes Unknown("")
        assert_eq!(
            input,
            ChatInput::Command(ChatCommand::Unknown(String::new()))
        );
    }
}
