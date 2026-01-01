//! Async XMODEM transfer implementation.
//!
//! Provides async functions for sending and receiving files using the XMODEM protocol.
//! This is a custom implementation that works with tokio's async I/O.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Result type for transfer operations.
pub type TransferResult<T> = std::result::Result<T, TransferError>;

/// Errors that can occur during XMODEM transfer.
#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    /// I/O error during transfer.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// XMODEM protocol error.
    #[error("XMODEM error: {0}")]
    Protocol(String),

    /// Transfer was cancelled by receiver.
    #[error("Transfer cancelled")]
    Cancelled,

    /// Timeout during transfer.
    #[error("Transfer timeout")]
    Timeout,

    /// Too many retries.
    #[error("Max retries exceeded")]
    MaxRetries,
}

// XMODEM control characters
const SOH: u8 = 0x01; // Start of header (128-byte block)
const EOT: u8 = 0x04; // End of transmission
const ACK: u8 = 0x06; // Acknowledge
const NAK: u8 = 0x15; // Negative acknowledge
const CAN: u8 = 0x18; // Cancel
const SUB: u8 = 0x1A; // Substitute (padding character)

/// Block size for standard XMODEM
const BLOCK_SIZE: usize = 128;

/// Maximum number of retries for a single block
const MAX_RETRIES: usize = 10;

/// Timeout for waiting for response
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for initial NAK from receiver
const INITIAL_TIMEOUT: Duration = Duration::from_secs(120);

/// Timeout for waiting for first byte from sender (short, for retry loop)
const START_BYTE_TIMEOUT: Duration = Duration::from_secs(3);

/// Number of times to send 'C' waiting for sender to start
const START_RETRIES: usize = 40; // 40 * 3 seconds = 120 seconds total

/// Send data using XMODEM protocol.
///
/// This function sends data to the remote end using XMODEM protocol.
/// The receiver should be waiting to receive with XMODEM.
///
/// # Arguments
///
/// * `stream` - The TCP stream to use for transfer
/// * `data` - The data to send
///
/// # Returns
///
/// The number of bytes sent on success.
pub async fn xmodem_send(stream: &mut TcpStream, data: &[u8]) -> TransferResult<usize> {
    // Wait for initial NAK from receiver (indicating they're ready)
    let start_byte = wait_for_start(stream).await?;
    let use_crc = start_byte == b'C';

    // Send data in blocks
    let mut block_num: u8 = 1;
    let mut offset = 0;

    while offset < data.len() {
        // Prepare block data (pad with SUB if necessary)
        let mut block = [SUB; BLOCK_SIZE];
        let end = (offset + BLOCK_SIZE).min(data.len());
        let len = end - offset;
        block[..len].copy_from_slice(&data[offset..end]);

        // Send block with retries
        send_block(stream, block_num, &block, use_crc).await?;

        block_num = block_num.wrapping_add(1);
        offset += BLOCK_SIZE;
    }

    // Send EOT
    send_eot(stream).await?;

    Ok(data.len())
}

/// Receive data using XMODEM protocol.
///
/// This function receives data from the remote end using XMODEM protocol.
/// The sender should be waiting to send with XMODEM.
///
/// # Arguments
///
/// * `stream` - The TCP stream to use for transfer
///
/// # Returns
///
/// The received data on success.
pub async fn xmodem_receive(stream: &mut TcpStream) -> TransferResult<Vec<u8>> {
    let mut data = Vec::new();
    let mut expected_block: u8 = 1;

    // Wait for first block with retry loop (send 'C' repeatedly until sender responds)
    let first_header = wait_for_sender_start(stream).await?;

    // Process first header
    let mut header = first_header;

    loop {
        match header {
            SOH => {
                // Receive block
                let (block_num, block_data) = receive_block(stream, true).await?;

                if block_num == expected_block {
                    data.extend_from_slice(&block_data);
                    expected_block = expected_block.wrapping_add(1);
                    stream.write_all(&[ACK]).await?;
                    stream.flush().await?;
                } else if block_num == expected_block.wrapping_sub(1) {
                    // Duplicate block, ACK but don't add data
                    stream.write_all(&[ACK]).await?;
                    stream.flush().await?;
                } else {
                    // Unexpected block number
                    stream.write_all(&[NAK]).await?;
                    stream.flush().await?;
                }
            }
            EOT => {
                // End of transmission
                stream.write_all(&[ACK]).await?;
                stream.flush().await?;
                break;
            }
            CAN => {
                return Err(TransferError::Cancelled);
            }
            _ => {
                // Unknown header, send NAK
                stream.write_all(&[NAK]).await?;
                stream.flush().await?;
            }
        }

        // Read next header
        header = match timeout(RESPONSE_TIMEOUT, read_byte(stream)).await {
            Ok(Ok(b)) => b,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(TransferError::Timeout),
        };
    }

    // Remove padding (SUB characters at the end)
    while data.last() == Some(&SUB) {
        data.pop();
    }

    Ok(data)
}

/// Wait for sender to start by sending 'C' repeatedly.
/// Returns the first valid header byte (SOH or EOT).
async fn wait_for_sender_start(stream: &mut TcpStream) -> TransferResult<u8> {
    // Telnet IAC (Interpret As Command) byte
    const IAC: u8 = 0xFF;

    for retry in 0..START_RETRIES {
        // Send 'C' for CRC mode
        stream.write_all(&[b'C']).await?;
        stream.flush().await?;

        // Try to read bytes within the timeout, skipping Telnet IAC sequences
        let start = std::time::Instant::now();
        while start.elapsed() < START_BYTE_TIMEOUT {
            let remaining = START_BYTE_TIMEOUT - start.elapsed();
            match timeout(remaining, read_byte(stream)).await {
                Ok(Ok(SOH)) => return Ok(SOH),
                Ok(Ok(EOT)) => return Ok(EOT),
                Ok(Ok(CAN)) => return Err(TransferError::Cancelled),
                Ok(Ok(IAC)) => {
                    // Telnet IAC sequence - read and skip the next 1-2 bytes
                    if let Ok(Ok(cmd)) = timeout(Duration::from_millis(100), read_byte(stream)).await
                    {
                        // If it's WILL/WONT/DO/DONT (0xFB-0xFE), read one more byte
                        if (0xFB..=0xFE).contains(&cmd) {
                            let _ = timeout(Duration::from_millis(100), read_byte(stream)).await;
                        }
                        // For SB (0xFA), we'd need to read until SE, but skip for now
                    }
                    continue;
                }
                Ok(Ok(b)) if b < 32 && b != SOH && b != EOT && b != CAN => {
                    // Other control characters - ignore
                    continue;
                }
                Ok(Ok(_)) => {
                    // Other printable bytes - might be echo, ignore
                    continue;
                }
                Ok(Err(e)) => return Err(e.into()),
                Err(_) => break, // Timeout on this read, send 'C' again
            }
        }

        tracing::debug!("XMODEM: Retry {} - sending 'C' again", retry + 1);
    }

    Err(TransferError::Timeout)
}

/// Wait for the initial start byte from receiver (NAK or 'C').
async fn wait_for_start(stream: &mut TcpStream) -> TransferResult<u8> {
    match timeout(INITIAL_TIMEOUT, async {
        loop {
            let byte = read_byte(stream).await?;
            match byte {
                NAK => return Ok(NAK),
                b'C' => return Ok(b'C'),
                CAN => return Err(TransferError::Cancelled),
                _ => continue, // Ignore other bytes
            }
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err(TransferError::Timeout),
    }
}

/// Send a single block with retries.
async fn send_block(
    stream: &mut TcpStream,
    block_num: u8,
    data: &[u8; BLOCK_SIZE],
    use_crc: bool,
) -> TransferResult<()> {
    for _ in 0..MAX_RETRIES {
        // Build packet
        let mut packet = Vec::with_capacity(BLOCK_SIZE + 5);
        packet.push(SOH);
        packet.push(block_num);
        packet.push(!block_num);
        packet.extend_from_slice(data);

        if use_crc {
            let crc = calculate_crc16(data);
            packet.push((crc >> 8) as u8);
            packet.push((crc & 0xFF) as u8);
        } else {
            let checksum = calculate_checksum(data);
            packet.push(checksum);
        }

        // Send packet
        stream.write_all(&packet).await?;
        stream.flush().await?;

        // Wait for response
        match timeout(RESPONSE_TIMEOUT, read_byte(stream)).await {
            Ok(Ok(ACK)) => return Ok(()),
            Ok(Ok(NAK)) => continue, // Retry
            Ok(Ok(CAN)) => return Err(TransferError::Cancelled),
            Ok(Ok(_)) => continue,   // Unknown response, retry
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => continue, // Timeout, retry
        }
    }

    Err(TransferError::MaxRetries)
}

/// Receive a single block.
async fn receive_block(
    stream: &mut TcpStream,
    use_crc: bool,
) -> TransferResult<(u8, [u8; BLOCK_SIZE])> {
    // Read block number and complement
    let block_num = read_byte(stream).await?;
    let block_num_complement = read_byte(stream).await?;

    // Verify complement
    if block_num != !block_num_complement {
        return Err(TransferError::Protocol(
            "Block number complement mismatch".to_string(),
        ));
    }

    // Read data
    let mut data = [0u8; BLOCK_SIZE];
    stream.read_exact(&mut data).await?;

    // Read and verify checksum/CRC
    if use_crc {
        let mut crc_bytes = [0u8; 2];
        stream.read_exact(&mut crc_bytes).await?;
        let received_crc = ((crc_bytes[0] as u16) << 8) | (crc_bytes[1] as u16);
        let calculated_crc = calculate_crc16(&data);
        if received_crc != calculated_crc {
            return Err(TransferError::Protocol("CRC mismatch".to_string()));
        }
    } else {
        let received_checksum = read_byte(stream).await?;
        let calculated_checksum = calculate_checksum(&data);
        if received_checksum != calculated_checksum {
            return Err(TransferError::Protocol("Checksum mismatch".to_string()));
        }
    }

    Ok((block_num, data))
}

/// Send EOT and wait for ACK.
async fn send_eot(stream: &mut TcpStream) -> TransferResult<()> {
    for _ in 0..MAX_RETRIES {
        stream.write_all(&[EOT]).await?;
        stream.flush().await?;

        match timeout(RESPONSE_TIMEOUT, read_byte(stream)).await {
            Ok(Ok(ACK)) => return Ok(()),
            Ok(Ok(NAK)) => continue,
            Ok(Ok(_)) => continue,  // Unknown response, retry
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => continue,
        }
    }

    Err(TransferError::MaxRetries)
}

/// Read a single byte from the stream.
async fn read_byte(stream: &mut TcpStream) -> std::io::Result<u8> {
    let mut buf = [0u8; 1];
    stream.read_exact(&mut buf).await?;
    Ok(buf[0])
}

/// Calculate simple checksum (sum of all bytes, mod 256).
fn calculate_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Calculate CRC-16/XMODEM.
fn calculate_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_error_display() {
        let err = TransferError::Cancelled;
        assert_eq!(err.to_string(), "Transfer cancelled");

        let err = TransferError::Timeout;
        assert_eq!(err.to_string(), "Transfer timeout");

        let err = TransferError::MaxRetries;
        assert_eq!(err.to_string(), "Max retries exceeded");
    }

    #[test]
    fn test_calculate_checksum() {
        assert_eq!(calculate_checksum(&[0, 0, 0]), 0);
        assert_eq!(calculate_checksum(&[1, 2, 3]), 6);
        assert_eq!(calculate_checksum(&[255]), 255);
        assert_eq!(calculate_checksum(&[200, 100]), 44); // 300 mod 256 = 44
    }

    #[test]
    fn test_calculate_crc16() {
        // Known CRC-16/XMODEM test vectors
        assert_eq!(calculate_crc16(b"123456789"), 0x31C3);
        assert_eq!(calculate_crc16(&[]), 0x0000);
    }
}
