//! This module contains helper functions for serializing, deserializing, sending and receiving data.

use std::net::{TcpStream, ToSocketAddrs};
use std::io::{Write, Read, ErrorKind};

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bincode::{deserialize, serialize};

use crate::nc_error::NCError;

/// Encode data to a Vec<u8>.
pub fn nc_encode_data<S: Serialize>(data: &S) -> Result<Vec<u8>, NCError> {
    serialize(data).map_err(|e| NCError::Serialize(e))
}

/// Decode data from a &[u8] slice to the type T.
pub fn nc_decode_data<'de, D: Deserialize<'de>>(data: &'de [u8]) -> Result<D, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

/// Decode data from an owened reference &[u8] to the type T.
pub fn nc_decode_data2<D: DeserializeOwned>(data: &[u8]) -> Result<D, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

/// Open a tcp connection and sends the data using the nc_send_data2() function.
pub(crate) fn nc_send_data<S: Serialize, A: ToSocketAddrs>(data: &S, socket_addr: &A) -> Result<(), NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)
}

/// Serialize the data and send it to the given Writer (usually a tcp stream).
pub(crate) fn nc_send_data2<S: Serialize, W: Write>(data: &S, tcp_stream: &mut W) -> Result<(), NCError> {
    let data = nc_encode_data(data)?;
    let data_len = data.len() as u64;

    let n = tcp_stream.write(&data_len.to_le_bytes())?;

    if n == 0 {
        // Connection closed by the other side.
        return Err(NCError::ConnectionClosed)
    } else {
        assert_eq!(n, 8);
    }

    let n = tcp_stream.write(&data)?;

    if n == 0 {
        // Connection closed by the other side.
        return Err(NCError::ConnectionClosed)
    } else {
        assert_eq!(n as u64, data_len);
    }

    tcp_stream.flush().map_err(|e| e.into())
}

/// Read data from the given Reader (usually a tcp stream) and deserialize it.
/// Return the deserialized data as a Result.
pub(crate) fn nc_receive_data<D: DeserializeOwned, R: Read>(tcp_stream: &mut R) -> Result<D, NCError> {
    const BUFFER_SIZE: usize = 1024 * 128;

    let mut data: Vec<u8> = Vec::new();
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut data_len: [u8; 8] = [0; 8];

    let n = tcp_stream.read(&mut data_len)?;

    if n == 0 {
        // Socket has been closed by the other side.
        return Err(NCError::ConnectionClosed)
    } else {
        assert_eq!(n, 8);
    }

    let data_len = u64::from_le_bytes(data_len);

    loop {
        match tcp_stream.read(&mut buffer) {
            Ok(n) => {
                if n == 0 {
                    // No more data available, socket has been closed by the other side.
                    break
                } else {
                    data.extend_from_slice(&buffer[0..n]);
                    if data.len() as u64 == data_len {
                        break
                    }
                }
            }
            Err(e) => {
                if !(e.kind() == ErrorKind::Interrupted) {
                    return Err(NCError::IOError(e))
                }
            }
        }
    }

    let result = nc_decode_data(&data)?;
    Ok(result)
}

/// Open a tcp stream, send the data using the nc_send_data2() function and receive data using the nc_receive_data() function.
/// Return the deserialized data as a Result.
pub(crate) fn nc_send_receive_data<S: Serialize, D: DeserializeOwned, A: ToSocketAddrs>(data: &S, socket_addr: &A) -> Result<D, NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;

    nc_send_data2(data, &mut tcp_stream)?;
    nc_receive_data(&mut tcp_stream)
}
