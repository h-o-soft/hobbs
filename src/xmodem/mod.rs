//! XMODEM file transfer protocol implementation.
//!
//! This module provides XMODEM file transfer capabilities for the BBS.

mod transfer;

pub use transfer::{xmodem_receive, xmodem_send, TransferError, TransferResult};
