//! Input bridge for connecting async TelnetSession with sync Lua callbacks.
//!
//! This module provides a channel-based bridge that allows synchronous Lua
//! callbacks to request input from an async TelnetSession.

use std::sync::mpsc::{self, Receiver, Sender};

/// A request for input from the script.
#[derive(Debug)]
pub struct InputRequest {
    /// The prompt to display (if any).
    pub prompt: Option<String>,
}

/// The script-side handle for requesting input.
///
/// This is used inside Lua callbacks to request input from the async context.
pub struct ScriptInputHandle {
    request_tx: Sender<InputRequest>,
    response_rx: Receiver<Option<String>>,
}

impl ScriptInputHandle {
    /// Request input with an optional prompt.
    ///
    /// This will block until the async handler provides a response.
    pub fn request_input(&self, prompt: Option<String>) -> Option<String> {
        // Send the request
        if self.request_tx.send(InputRequest { prompt }).is_err() {
            // Channel closed - return None
            return None;
        }

        // Wait for response (blocking)
        self.response_rx.recv().ok().flatten()
    }
}

/// The async-side handle for handling input requests.
///
/// This is used in the async context to receive input requests and provide responses.
pub struct AsyncInputHandle {
    request_rx: Receiver<InputRequest>,
    response_tx: Sender<Option<String>>,
}

impl AsyncInputHandle {
    /// Try to receive an input request without blocking.
    ///
    /// Returns `Some(request)` if there's a pending request, `None` otherwise.
    pub fn try_recv(&self) -> Option<InputRequest> {
        self.request_rx.try_recv().ok()
    }

    /// Receive an input request, blocking until one is available.
    ///
    /// Returns `None` if the channel is closed.
    pub fn recv(&self) -> Option<InputRequest> {
        self.request_rx.recv().ok()
    }

    /// Receive an input request with a timeout.
    ///
    /// Returns `None` if the timeout expires or the channel is closed.
    pub fn recv_timeout(&self, timeout: std::time::Duration) -> Option<InputRequest> {
        self.request_rx.recv_timeout(timeout).ok()
    }

    /// Send a response back to the script.
    ///
    /// Returns `true` if the response was sent successfully.
    pub fn send_response(&self, response: Option<String>) -> bool {
        self.response_tx.send(response).is_ok()
    }
}

/// Create a new input bridge pair.
///
/// Returns a tuple of (script_handle, async_handle).
pub fn create_input_bridge() -> (ScriptInputHandle, AsyncInputHandle) {
    let (request_tx, request_rx) = mpsc::channel();
    let (response_tx, response_rx) = mpsc::channel();

    let script_handle = ScriptInputHandle {
        request_tx,
        response_rx,
    };

    let async_handle = AsyncInputHandle {
        request_rx,
        response_tx,
    };

    (script_handle, async_handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_input_bridge_basic() {
        let (script_handle, async_handle) = create_input_bridge();

        // Simulate async handler in a thread
        let handler_thread = thread::spawn(move || {
            // Wait for request
            let request = async_handle.recv().unwrap();
            assert_eq!(request.prompt, Some("Enter name: ".to_string()));

            // Send response
            async_handle.send_response(Some("TestUser".to_string()));
        });

        // Script-side request
        let result = script_handle.request_input(Some("Enter name: ".to_string()));
        assert_eq!(result, Some("TestUser".to_string()));

        handler_thread.join().unwrap();
    }

    #[test]
    fn test_input_bridge_no_prompt() {
        let (script_handle, async_handle) = create_input_bridge();

        let handler_thread = thread::spawn(move || {
            let request = async_handle.recv().unwrap();
            assert!(request.prompt.is_none());
            async_handle.send_response(Some("input".to_string()));
        });

        let result = script_handle.request_input(None);
        assert_eq!(result, Some("input".to_string()));

        handler_thread.join().unwrap();
    }

    #[test]
    fn test_input_bridge_channel_closed() {
        let (script_handle, async_handle) = create_input_bridge();

        // Drop the async handle to close the channel
        drop(async_handle);

        // Request should return None
        let result = script_handle.request_input(None);
        assert!(result.is_none());
    }
}
