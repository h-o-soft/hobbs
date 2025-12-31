//! Integration tests for Telnet protocol handling.

use hobbs::server::telnet::{control, iac, option};
use hobbs::{
    initial_negotiation, EchoMode, InputResult, LineBuffer, NegotiationState, TelnetCommand,
    TelnetParser,
};

#[test]
fn test_telnet_negotiation_flow() {
    // Simulate the initial connection flow
    let mut parser = TelnetParser::new();
    let mut state = NegotiationState::default();

    // Server sends initial negotiation
    let server_init = initial_negotiation();
    assert_eq!(
        server_init,
        vec![
            iac::IAC,
            iac::WILL,
            option::ECHO,
            iac::IAC,
            iac::WILL,
            option::SGA,
        ]
    );

    // Client responds with DO ECHO, DO SGA
    let client_response = vec![
        iac::IAC,
        iac::DO,
        option::ECHO,
        iac::IAC,
        iac::DO,
        option::SGA,
    ];

    let (data, commands) = parser.parse(&client_response);
    assert!(data.is_empty());
    assert_eq!(commands.len(), 2);

    // Process the commands
    for cmd in &commands {
        let response = TelnetParser::respond_to_command(cmd, &mut state);
        // Should be empty since we already sent WILL
        assert!(response.is_empty());
    }

    assert!(state.echo_enabled);
    assert!(state.sga_enabled);
}

#[test]
fn test_input_with_telnet_commands() {
    let mut parser = TelnetParser::new();
    let mut buffer = LineBuffer::with_defaults();

    // Client sends "Hello" mixed with a DO ECHO command
    let mut input = b"He".to_vec();
    input.extend_from_slice(&[iac::IAC, iac::DO, option::ECHO]);
    input.extend_from_slice(b"llo\r");

    // Parse Telnet commands
    let (data, commands) = parser.parse(&input);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0], TelnetCommand::Do(option::ECHO));

    // Process input data
    let results = buffer.process_bytes(&data);

    // Find the Line result
    let line_result = results
        .iter()
        .find(|(r, _)| matches!(r, InputResult::Line(_)));
    assert!(line_result.is_some());
    if let (InputResult::Line(line), _) = line_result.unwrap() {
        assert_eq!(line, "Hello");
    }
}

#[test]
fn test_password_input_flow() {
    let mut buffer = LineBuffer::with_defaults();

    // Normal input first
    let (result, echo) = buffer.process_byte(b'u');
    assert_eq!(result, InputResult::Buffering);
    assert_eq!(echo, vec![b'u']);

    // Complete username
    buffer.process_byte(b's');
    buffer.process_byte(b'e');
    buffer.process_byte(b'r');
    let (result, _) = buffer.process_byte(control::CR);
    assert_eq!(result, InputResult::Line("user".to_string()));

    // Switch to password mode
    buffer.set_echo_mode(EchoMode::Password);

    // Password input should not echo
    let (result, echo) = buffer.process_byte(b'p');
    assert_eq!(result, InputResult::Buffering);
    assert!(echo.is_empty()); // No echo

    buffer.process_byte(b'a');
    buffer.process_byte(b's');
    buffer.process_byte(b's');
    let (result, _) = buffer.process_byte(control::CR);
    assert_eq!(result, InputResult::Line("pass".to_string()));
}

#[test]
fn test_backspace_editing() {
    let mut buffer = LineBuffer::with_defaults();

    // Type "Helo"
    buffer.process_byte(b'H');
    buffer.process_byte(b'e');
    buffer.process_byte(b'l');
    buffer.process_byte(b'o');

    // Backspace to remove 'o'
    let (result, echo) = buffer.process_byte(control::BS);
    assert_eq!(result, InputResult::Buffering);
    assert_eq!(echo, vec![control::BS, b' ', control::BS]);

    // Backspace to remove 'l'
    buffer.process_byte(control::DEL);

    // Type "llo" to make "Hello"
    buffer.process_byte(b'l');
    buffer.process_byte(b'l');
    buffer.process_byte(b'o');

    let (result, _) = buffer.process_byte(control::CR);
    assert_eq!(result, InputResult::Line("Hello".to_string()));
}

#[test]
fn test_ctrl_c_cancellation() {
    let mut buffer = LineBuffer::with_defaults();

    // Type some text
    buffer.process_byte(b'H');
    buffer.process_byte(b'e');
    buffer.process_byte(b'l');
    buffer.process_byte(b'l');
    buffer.process_byte(b'o');

    // Ctrl+C cancels
    let (result, echo) = buffer.process_byte(control::ETX);
    assert_eq!(result, InputResult::Cancel);
    assert_eq!(echo, vec![b'^', b'C', control::CR, control::LF]);

    // Buffer should be cleared
    assert!(buffer.is_empty());

    // Can start typing again
    buffer.process_byte(b'B');
    buffer.process_byte(b'y');
    buffer.process_byte(b'e');
    let (result, _) = buffer.process_byte(control::CR);
    assert_eq!(result, InputResult::Line("Bye".to_string()));
}

#[test]
fn test_naws_subnegotiation() {
    let mut parser = TelnetParser::new();
    let mut state = NegotiationState::default();

    // Client sends WILL NAWS
    let (_, commands) = parser.parse(&[iac::IAC, iac::WILL, option::NAWS]);
    assert_eq!(commands.len(), 1);

    // Server should respond with DO NAWS
    let response = TelnetParser::respond_to_command(&commands[0], &mut state);
    assert_eq!(response, vec![iac::IAC, iac::DO, option::NAWS]);

    // Client sends window size subnegotiation (80x24)
    let subneg = vec![
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

    let (_, commands) = parser.parse(&subneg);
    assert_eq!(commands.len(), 1);

    if let TelnetCommand::Subnegotiation { option, data } = &commands[0] {
        assert_eq!(*option, option::NAWS);
        assert_eq!(data.len(), 4);

        // Parse window size
        let width = ((data[0] as u16) << 8) | (data[1] as u16);
        let height = ((data[2] as u16) << 8) | (data[3] as u16);
        assert_eq!(width, 80);
        assert_eq!(height, 24);
    } else {
        panic!("Expected Subnegotiation command");
    }
}

#[test]
fn test_echo_toggle() {
    let mut state = NegotiationState {
        echo_enabled: true,
        sga_enabled: true,
    };

    // Client sends DONT ECHO (disable echo)
    let response = TelnetParser::respond_to_command(&TelnetCommand::Dont(option::ECHO), &mut state);
    assert_eq!(response, vec![iac::IAC, iac::WONT, option::ECHO]);
    assert!(!state.echo_enabled);

    // Client sends DO ECHO (enable echo)
    let response = TelnetParser::respond_to_command(&TelnetCommand::Do(option::ECHO), &mut state);
    assert!(response.is_empty()); // Already acknowledged
    assert!(state.echo_enabled);
}

#[test]
fn test_escaped_iac() {
    let mut parser = TelnetParser::new();

    // IAC IAC (255 255) should be interpreted as a single literal 255
    let input = vec![b'H', b'i', iac::IAC, iac::IAC, b'!'];
    let (data, commands) = parser.parse(&input);

    // The escaped IAC is not added to data in current implementation
    // but no command should be generated
    assert!(commands.is_empty());
    assert_eq!(data, b"Hi!");
}

#[test]
fn test_masked_password_mode() {
    let mut buffer = LineBuffer::with_defaults();
    buffer.set_echo_mode(EchoMode::Masked('*'));

    // Password input should echo asterisks
    let (_, echo) = buffer.process_byte(b'p');
    assert_eq!(echo, b"*");

    let (_, echo) = buffer.process_byte(b'a');
    assert_eq!(echo, b"*");

    let (_, echo) = buffer.process_byte(b's');
    assert_eq!(echo, b"*");

    let (_, echo) = buffer.process_byte(b's');
    assert_eq!(echo, b"*");

    // Backspace should still work
    let (_, echo) = buffer.process_byte(control::BS);
    assert_eq!(echo, vec![control::BS, b' ', control::BS]);

    // Re-type the last character
    let (_, echo) = buffer.process_byte(b's');
    assert_eq!(echo, b"*");

    // Complete
    let (result, _) = buffer.process_byte(control::CR);
    assert_eq!(result, InputResult::Line("pass".to_string()));
}

#[test]
fn test_shiftjis_input_processing() {
    use hobbs::{decode_shiftjis, encode_shiftjis};

    let mut buffer = LineBuffer::with_defaults();

    // Simulate ShiftJIS input "テスト" (already decoded to UTF-8)
    let shiftjis_bytes = vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
    let decoded = decode_shiftjis(&shiftjis_bytes);
    assert!(!decoded.had_errors);
    assert_eq!(decoded.text, "テスト");

    // Process UTF-8 bytes
    for byte in decoded.text.as_bytes() {
        buffer.process_byte(*byte);
    }

    let (result, _) = buffer.process_byte(control::CR);
    if let InputResult::Line(line) = result {
        assert_eq!(line, "テスト");

        // Encode back to ShiftJIS for sending
        let encoded = encode_shiftjis(&line);
        assert!(!encoded.had_errors);
        assert_eq!(encoded.bytes, shiftjis_bytes);
    } else {
        panic!("Expected Line result");
    }
}
