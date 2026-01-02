//! Telnet protocol implementation.
//!
//! This module provides Telnet protocol constants, commands, and negotiation
//! handling according to RFC 854.

/// Telnet command bytes (IAC = Interpret As Command).
pub mod iac {
    /// IAC - Interpret As Command (255)
    pub const IAC: u8 = 255;

    /// WILL - Sender wants to enable option (251)
    pub const WILL: u8 = 251;

    /// WONT - Sender refuses to enable option (252)
    pub const WONT: u8 = 252;

    /// DO - Sender wants receiver to enable option (253)
    pub const DO: u8 = 253;

    /// DONT - Sender wants receiver to disable option (254)
    pub const DONT: u8 = 254;

    /// SB - Subnegotiation Begin (250)
    pub const SB: u8 = 250;

    /// SE - Subnegotiation End (240)
    pub const SE: u8 = 240;

    /// NOP - No Operation (241)
    pub const NOP: u8 = 241;

    /// GA - Go Ahead (249)
    pub const GA: u8 = 249;
}

/// Telnet option codes.
pub mod option {
    /// ECHO - Echo option (1)
    pub const ECHO: u8 = 1;

    /// SGA - Suppress Go Ahead (3)
    pub const SGA: u8 = 3;

    /// TERMINAL_TYPE - Terminal Type (24)
    pub const TERMINAL_TYPE: u8 = 24;

    /// NAWS - Negotiate About Window Size (31)
    pub const NAWS: u8 = 31;
}

/// Control characters used in Telnet communication.
pub mod control {
    /// NUL - Null character
    pub const NUL: u8 = 0x00;

    /// ETX - End of Text (Ctrl+C)
    pub const ETX: u8 = 0x03;

    /// EOT - End of Transmission (Ctrl+D)
    pub const EOT: u8 = 0x04;

    /// BS - Backspace
    pub const BS: u8 = 0x08;

    /// LF - Line Feed
    pub const LF: u8 = 0x0A;

    /// CR - Carriage Return
    pub const CR: u8 = 0x0D;

    /// ESC - Escape
    pub const ESC: u8 = 0x1B;

    /// DEL - Delete (also used as backspace)
    pub const DEL: u8 = 0x7F;
}

/// Telnet negotiation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NegotiationState {
    /// Whether ECHO is enabled (server echoes input back to client).
    pub echo_enabled: bool,
    /// Whether SGA (Suppress Go Ahead) is enabled.
    pub sga_enabled: bool,
}

/// Generate the initial negotiation bytes to send to the client.
///
/// This returns the bytes for:
/// - IAC WILL ECHO (server will echo)
/// - IAC WILL SGA (server will suppress go-ahead)
///
/// # Returns
///
/// A vector of bytes to send to the client.
pub fn initial_negotiation() -> Vec<u8> {
    vec![
        iac::IAC,
        iac::WILL,
        option::ECHO,
        iac::IAC,
        iac::WILL,
        option::SGA,
    ]
}

/// Generate bytes to enable server echo.
pub fn enable_echo() -> Vec<u8> {
    vec![iac::IAC, iac::WILL, option::ECHO]
}

/// Generate bytes to disable server echo.
pub fn disable_echo() -> Vec<u8> {
    vec![iac::IAC, iac::WONT, option::ECHO]
}

/// Result of parsing IAC commands from input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    /// Regular data bytes (not IAC commands).
    Data(Vec<u8>),
    /// IAC command received.
    Command(TelnetCommand),
    /// Need more data to complete parsing.
    Incomplete,
}

/// A parsed Telnet command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelnetCommand {
    /// WILL option
    Will(u8),
    /// WONT option
    Wont(u8),
    /// DO option
    Do(u8),
    /// DONT option
    Dont(u8),
    /// Subnegotiation data
    Subnegotiation { option: u8, data: Vec<u8> },
    /// NOP
    Nop,
    /// Go Ahead
    GoAhead,
}

/// Parser for Telnet protocol data.
#[derive(Debug, Default)]
pub struct TelnetParser {
    /// Buffer for incomplete IAC sequences.
    buffer: Vec<u8>,
    /// Whether we're in the middle of an IAC sequence.
    in_iac: bool,
    /// Whether we're in a subnegotiation.
    in_subneg: bool,
    /// Subnegotiation option being parsed.
    subneg_option: u8,
    /// Subnegotiation data buffer.
    subneg_data: Vec<u8>,
}

impl TelnetParser {
    /// Create a new Telnet parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse input bytes and separate data from IAC commands.
    ///
    /// Returns a tuple of (data bytes, commands).
    pub fn parse(&mut self, input: &[u8]) -> (Vec<u8>, Vec<TelnetCommand>) {
        let mut data = Vec::new();
        let mut commands = Vec::new();

        for &byte in input {
            if self.in_subneg {
                self.parse_subneg_byte(byte, &mut commands);
            } else if self.in_iac {
                if let Some(data_byte) = self.parse_iac_byte(byte, &mut commands) {
                    data.push(data_byte);
                }
            } else if byte == iac::IAC {
                self.in_iac = true;
            } else {
                data.push(byte);
            }
        }

        (data, commands)
    }

    /// Parse a byte in IAC state.
    ///
    /// Returns Some(byte) if the byte should be added to data,
    /// None if it was consumed as part of a command.
    fn parse_iac_byte(&mut self, byte: u8, commands: &mut Vec<TelnetCommand>) -> Option<u8> {
        match byte {
            iac::IAC => {
                // Escaped IAC (255 255 = literal 255)
                self.in_iac = false;
                Some(0xFF) // Return the escaped 0xFF as data
            }
            iac::WILL => {
                self.buffer.push(byte);
                None
            }
            iac::WONT => {
                self.buffer.push(byte);
                None
            }
            iac::DO => {
                self.buffer.push(byte);
                None
            }
            iac::DONT => {
                self.buffer.push(byte);
                None
            }
            iac::SB => {
                self.in_subneg = true;
                self.in_iac = false;
                None
            }
            iac::NOP => {
                commands.push(TelnetCommand::Nop);
                self.in_iac = false;
                None
            }
            iac::GA => {
                commands.push(TelnetCommand::GoAhead);
                self.in_iac = false;
                None
            }
            _ => {
                if !self.buffer.is_empty() {
                    // This is the option byte for WILL/WONT/DO/DONT
                    let cmd = self.buffer.pop().unwrap();
                    let command = match cmd {
                        iac::WILL => TelnetCommand::Will(byte),
                        iac::WONT => TelnetCommand::Wont(byte),
                        iac::DO => TelnetCommand::Do(byte),
                        iac::DONT => TelnetCommand::Dont(byte),
                        _ => {
                            // Invalid command, treat as data
                            self.in_iac = false;
                            self.buffer.clear();
                            tracing::warn!(
                                "Invalid Telnet command sequence: IAC {:02X} {:02X}",
                                cmd,
                                byte
                            );
                            return Some(byte);
                        }
                    };
                    commands.push(command);
                    self.in_iac = false;
                    self.buffer.clear();
                    None
                } else {
                    // IAC followed by unexpected byte - treat as data
                    // This can happen if the client sends malformed data
                    self.in_iac = false;
                    tracing::warn!("Unexpected byte after IAC: {:02X}, treating as data", byte);
                    Some(byte)
                }
            }
        }
    }

    fn parse_subneg_byte(&mut self, byte: u8, commands: &mut Vec<TelnetCommand>) {
        if byte == iac::IAC {
            self.in_iac = true;
        } else if self.in_iac && byte == iac::SE {
            // End of subnegotiation
            commands.push(TelnetCommand::Subnegotiation {
                option: self.subneg_option,
                data: std::mem::take(&mut self.subneg_data),
            });
            self.in_subneg = false;
            self.in_iac = false;
            self.subneg_option = 0;
        } else if self.in_iac {
            // IAC IAC in subnegotiation = literal 255
            if byte == iac::IAC {
                self.subneg_data.push(255);
            }
            self.in_iac = false;
        } else if self.subneg_data.is_empty() && self.subneg_option == 0 {
            // First byte is the option
            self.subneg_option = byte;
        } else {
            self.subneg_data.push(byte);
        }
    }

    /// Generate a response for a received command.
    ///
    /// Returns bytes to send back to the client, if any.
    pub fn respond_to_command(command: &TelnetCommand, state: &mut NegotiationState) -> Vec<u8> {
        match command {
            TelnetCommand::Do(opt) => {
                match *opt {
                    option::ECHO => {
                        state.echo_enabled = true;
                        // Already sent WILL ECHO, no need to respond
                        vec![]
                    }
                    option::SGA => {
                        state.sga_enabled = true;
                        // Already sent WILL SGA, no need to respond
                        vec![]
                    }
                    _ => {
                        // We don't support this option
                        vec![iac::IAC, iac::WONT, *opt]
                    }
                }
            }
            TelnetCommand::Dont(opt) => match *opt {
                option::ECHO => {
                    state.echo_enabled = false;
                    vec![iac::IAC, iac::WONT, option::ECHO]
                }
                option::SGA => {
                    state.sga_enabled = false;
                    vec![iac::IAC, iac::WONT, option::SGA]
                }
                _ => vec![],
            },
            TelnetCommand::Will(opt) => {
                // Client wants to enable an option
                match *opt {
                    option::NAWS => {
                        // Accept NAWS
                        vec![iac::IAC, iac::DO, option::NAWS]
                    }
                    _ => {
                        // We don't want the client to do this
                        vec![iac::IAC, iac::DONT, *opt]
                    }
                }
            }
            TelnetCommand::Wont(_) => {
                // Client refuses, that's fine
                vec![]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_negotiation() {
        let bytes = initial_negotiation();
        assert_eq!(
            bytes,
            vec![
                iac::IAC,
                iac::WILL,
                option::ECHO,
                iac::IAC,
                iac::WILL,
                option::SGA,
            ]
        );
    }

    #[test]
    fn test_enable_disable_echo() {
        assert_eq!(enable_echo(), vec![iac::IAC, iac::WILL, option::ECHO]);
        assert_eq!(disable_echo(), vec![iac::IAC, iac::WONT, option::ECHO]);
    }

    #[test]
    fn test_parse_plain_data() {
        let mut parser = TelnetParser::new();
        let (data, commands) = parser.parse(b"Hello, World!");
        assert_eq!(data, b"Hello, World!");
        assert!(commands.is_empty());
    }

    #[test]
    fn test_parse_do_echo() {
        let mut parser = TelnetParser::new();
        let input = vec![iac::IAC, iac::DO, option::ECHO];
        let (data, commands) = parser.parse(&input);
        assert!(data.is_empty());
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], TelnetCommand::Do(option::ECHO));
    }

    #[test]
    fn test_parse_will_sga() {
        let mut parser = TelnetParser::new();
        let input = vec![iac::IAC, iac::WILL, option::SGA];
        let (data, commands) = parser.parse(&input);
        assert!(data.is_empty());
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], TelnetCommand::Will(option::SGA));
    }

    #[test]
    fn test_parse_mixed_data_and_commands() {
        let mut parser = TelnetParser::new();
        let mut input = b"Hello".to_vec();
        input.extend_from_slice(&[iac::IAC, iac::DO, option::ECHO]);
        input.extend_from_slice(b"World");

        let (data, commands) = parser.parse(&input);
        assert_eq!(data, b"HelloWorld");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], TelnetCommand::Do(option::ECHO));
    }

    #[test]
    fn test_parse_nop() {
        let mut parser = TelnetParser::new();
        let input = vec![iac::IAC, iac::NOP];
        let (data, commands) = parser.parse(&input);
        assert!(data.is_empty());
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], TelnetCommand::Nop);
    }

    #[test]
    fn test_parse_subnegotiation() {
        let mut parser = TelnetParser::new();
        // IAC SB NAWS <data> IAC SE
        let input = vec![
            iac::IAC,
            iac::SB,
            option::NAWS,
            0x00,
            0x50, // width = 80
            0x00,
            0x18, // height = 24
            iac::IAC,
            iac::SE,
        ];
        let (data, commands) = parser.parse(&input);
        assert!(data.is_empty());
        assert_eq!(commands.len(), 1);
        if let TelnetCommand::Subnegotiation {
            option,
            data: subneg_data,
        } = &commands[0]
        {
            assert_eq!(*option, option::NAWS);
            assert_eq!(subneg_data, &[0x00, 0x50, 0x00, 0x18]);
        } else {
            panic!("Expected Subnegotiation command");
        }
    }

    #[test]
    fn test_respond_to_do_echo() {
        let mut state = NegotiationState::default();
        let response =
            TelnetParser::respond_to_command(&TelnetCommand::Do(option::ECHO), &mut state);
        assert!(response.is_empty()); // Already sent WILL
        assert!(state.echo_enabled);
    }

    #[test]
    fn test_respond_to_dont_echo() {
        let mut state = NegotiationState {
            echo_enabled: true,
            sga_enabled: true,
        };
        let response =
            TelnetParser::respond_to_command(&TelnetCommand::Dont(option::ECHO), &mut state);
        assert_eq!(response, vec![iac::IAC, iac::WONT, option::ECHO]);
        assert!(!state.echo_enabled);
    }

    #[test]
    fn test_respond_to_will_naws() {
        let mut state = NegotiationState::default();
        let response =
            TelnetParser::respond_to_command(&TelnetCommand::Will(option::NAWS), &mut state);
        assert_eq!(response, vec![iac::IAC, iac::DO, option::NAWS]);
    }

    #[test]
    fn test_respond_to_unknown_option() {
        let mut state = NegotiationState::default();
        let response = TelnetParser::respond_to_command(&TelnetCommand::Do(99), &mut state);
        assert_eq!(response, vec![iac::IAC, iac::WONT, 99]);
    }

    #[test]
    fn test_control_constants() {
        assert_eq!(control::CR, 0x0D);
        assert_eq!(control::LF, 0x0A);
        assert_eq!(control::BS, 0x08);
        assert_eq!(control::DEL, 0x7F);
        assert_eq!(control::ETX, 0x03);
        assert_eq!(control::EOT, 0x04);
        assert_eq!(control::ESC, 0x1B);
    }
}
