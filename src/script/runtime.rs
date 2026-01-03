//! Script runtime for bridging async I/O with sync Lua execution.
//!
//! This module provides a message-passing based runtime that allows
//! synchronous Lua scripts to communicate with the async TelnetSession.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::time::Duration;

/// Messages sent from the Lua script thread to the async host.
#[derive(Debug, Clone)]
pub enum ScriptMessage {
    /// Output text to display.
    Output(String),

    /// Request for user input.
    InputRequest {
        /// Optional prompt to display before reading input.
        prompt: Option<String>,
    },

    /// Script execution completed.
    Done {
        /// Whether execution was successful.
        success: bool,
        /// Error message if execution failed.
        error: Option<String>,
    },
}

/// Messages sent from the async host to the Lua script thread.
#[derive(Debug, Clone)]
pub enum HostMessage {
    /// Response to an input request.
    InputResponse(Option<String>),
}

/// Handle used by the Lua script thread to communicate with the host.
///
/// This is passed to the BBS API and used within Lua callbacks.
/// The Receiver is wrapped in a Mutex to make ScriptHandle Sync,
/// allowing Arc<ScriptHandle> to be Send for use across threads.
pub struct ScriptHandle {
    /// Channel to send messages to the host.
    script_tx: Sender<ScriptMessage>,
    /// Channel to receive messages from the host.
    /// Wrapped in Mutex to make ScriptHandle Sync.
    host_rx: Mutex<Receiver<HostMessage>>,
}

impl ScriptHandle {
    /// Send output text to the host for display.
    pub fn send_output(&self, text: String) {
        let _ = self.script_tx.send(ScriptMessage::Output(text));
    }

    /// Request input from the user.
    ///
    /// This will block until the host provides an input response.
    pub fn request_input(&self, prompt: Option<String>) -> Option<String> {
        // Send the input request
        if self
            .script_tx
            .send(ScriptMessage::InputRequest { prompt })
            .is_err()
        {
            return None;
        }

        // Wait for the response
        let rx = self.host_rx.lock().unwrap();
        match rx.recv() {
            Ok(HostMessage::InputResponse(input)) => input,
            Err(_) => None,
        }
    }

    /// Notify the host that script execution is complete.
    pub fn send_done(&self, success: bool, error: Option<String>) {
        let _ = self.script_tx.send(ScriptMessage::Done { success, error });
    }
}

/// Runtime used by the async host to communicate with the Lua script thread.
pub struct ScriptRuntime {
    /// Channel to receive messages from the script.
    script_rx: Receiver<ScriptMessage>,
    /// Channel to send messages to the script.
    host_tx: Sender<HostMessage>,
}

impl ScriptRuntime {
    /// Receive a message from the script.
    ///
    /// This blocks until a message is available.
    pub fn recv(&self) -> Option<ScriptMessage> {
        self.script_rx.recv().ok()
    }

    /// Try to receive a message from the script without blocking.
    pub fn try_recv(&self) -> Option<ScriptMessage> {
        self.script_rx.try_recv().ok()
    }

    /// Receive a message with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<ScriptMessage> {
        self.script_rx.recv_timeout(timeout).ok()
    }

    /// Send an input response to the script.
    pub fn send_input(&self, input: Option<String>) -> bool {
        self.host_tx.send(HostMessage::InputResponse(input)).is_ok()
    }
}

/// Create a new script runtime and handle pair.
///
/// Returns `(runtime, handle)` where:
/// - `runtime` is used by the async host to receive messages and send input responses
/// - `handle` is used by the Lua script thread to send output and request input
pub fn create_script_runtime() -> (ScriptRuntime, ScriptHandle) {
    // Channel for script -> host communication
    let (script_tx, script_rx) = mpsc::channel();
    // Channel for host -> script communication
    let (host_tx, host_rx) = mpsc::channel();

    let runtime = ScriptRuntime { script_rx, host_tx };
    let handle = ScriptHandle {
        script_tx,
        host_rx: Mutex::new(host_rx),
    };

    (runtime, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_output_message() {
        let (runtime, handle) = create_script_runtime();

        thread::spawn(move || {
            handle.send_output("Hello, World!".to_string());
            handle.send_done(true, None);
        });

        let msg = runtime.recv().unwrap();
        assert!(matches!(msg, ScriptMessage::Output(text) if text == "Hello, World!"));

        let msg = runtime.recv().unwrap();
        assert!(matches!(
            msg,
            ScriptMessage::Done {
                success: true,
                error: None
            }
        ));
    }

    #[test]
    fn test_input_request_response() {
        let (runtime, handle) = create_script_runtime();

        let script_thread = thread::spawn(move || {
            let input = handle.request_input(Some("Enter name: ".to_string()));
            assert_eq!(input, Some("TestUser".to_string()));
            handle.send_done(true, None);
        });

        // Receive input request
        let msg = runtime.recv().unwrap();
        assert!(
            matches!(msg, ScriptMessage::InputRequest { prompt: Some(p) } if p == "Enter name: ")
        );

        // Send response
        runtime.send_input(Some("TestUser".to_string()));

        // Wait for done
        let msg = runtime.recv().unwrap();
        assert!(matches!(msg, ScriptMessage::Done { success: true, .. }));

        script_thread.join().unwrap();
    }

    #[test]
    fn test_multiple_outputs() {
        let (runtime, handle) = create_script_runtime();

        thread::spawn(move || {
            handle.send_output("Line 1\n".to_string());
            handle.send_output("Line 2\n".to_string());
            handle.send_output("Line 3\n".to_string());
            handle.send_done(true, None);
        });

        let mut outputs = Vec::new();
        loop {
            match runtime.recv() {
                Some(ScriptMessage::Output(text)) => outputs.push(text),
                Some(ScriptMessage::Done { .. }) => break,
                _ => break,
            }
        }

        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0], "Line 1\n");
        assert_eq!(outputs[1], "Line 2\n");
        assert_eq!(outputs[2], "Line 3\n");
    }

    #[test]
    fn test_done_with_error() {
        let (runtime, handle) = create_script_runtime();

        thread::spawn(move || {
            handle.send_done(false, Some("Script error occurred".to_string()));
        });

        let msg = runtime.recv().unwrap();
        match msg {
            ScriptMessage::Done { success, error } => {
                assert!(!success);
                assert_eq!(error, Some("Script error occurred".to_string()));
            }
            _ => panic!("Expected Done message"),
        }
    }

    #[test]
    fn test_recv_timeout() {
        let (runtime, _handle) = create_script_runtime();

        // Should timeout since no message is sent
        let result = runtime.recv_timeout(Duration::from_millis(50));
        assert!(result.is_none());
    }

    #[test]
    fn test_try_recv() {
        let (runtime, handle) = create_script_runtime();

        // Initially empty
        assert!(runtime.try_recv().is_none());

        // Send a message
        handle.send_output("test".to_string());

        // Now should have a message
        let msg = runtime.try_recv();
        assert!(matches!(msg, Some(ScriptMessage::Output(text)) if text == "test"));
    }
}
