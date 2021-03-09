//! This module contains helper functions for serializing, deserializing, sending and receiving data.

use std::net::{TcpStream, ToSocketAddrs};
use std::io::{Write, Read};

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
    let data_len = data.len() as u64; // u64 is platform independend, usize is platform dependend

    tcp_stream.write_all(&data_len.to_le_bytes())?;
    tcp_stream.write_all(&data)?;
    tcp_stream.flush().map_err(|e| e.into())
}

/// Read data from the given Reader (usually a tcp stream) and deserialize it.
/// Return the deserialized data as a Result.
pub(crate) fn nc_receive_data<D: DeserializeOwned, R: Read>(tcp_stream: &mut R) -> Result<D, NCError> {
    let mut data_len: [u8; 8] = [0; 8];
    tcp_stream.read_exact(&mut data_len)?;
    let data_len = u64::from_le_bytes(data_len);  // u64 is platform independend, usize is platform dependend

    let mut data: Vec<u8> = vec![0; data_len as usize];
    tcp_stream.read_exact(&mut data)?;

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
