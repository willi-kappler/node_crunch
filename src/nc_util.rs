//! This module contains helper functions for serializing, deserializing, sending and receiving data.

use std::net::{TcpStream, ToSocketAddrs};
use std::io::{Write, Read};

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bincode::{deserialize, serialize, serialize_into, deserialize_from};

use crate::nc_error::NCError;

/// Encode data to a Vec<u8>
pub fn nc_encode_data<T: Serialize>(data: &T) -> Result<Vec<u8>, NCError> {
    serialize(data).map_err(|e| NCError::Serialize(e))
}

/// Decode data from a &[u8] slice to the type T.
pub fn nc_decode_data<'de, T: Deserialize<'de>>(data: &'de [u8]) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

/// Decode data from an owened reference &[u8] to the type T.
pub fn nc_decode_data2<T: DeserializeOwned>(data: &[u8]) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

/// Open a tcp connection and sends the data using the nc_send_data2() function.
pub(crate) fn nc_send_data<T: Serialize, A: ToSocketAddrs>(data: &T, socket_addr: &A) -> Result<(), NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)
}

/// Serialize the data and send it to the given Writer (usually a tcp stream).
pub(crate) fn nc_send_data2<S: Serialize, W: Write>(data: &S, tcp_stream: &mut W) -> Result<(), NCError> {
    serialize_into(tcp_stream, data)?;
    Ok(())
}

/// Read data from the given Reader (usually a tcp stream) and deserialize it.
/// Return the deserialized data as a Result
pub(crate) fn nc_receive_data<D: DeserializeOwned, R: Read>(tcp_stream: &mut R) -> Result<D, NCError> {
    let result = deserialize_from(tcp_stream)?;
    Ok(result)
}

/// Open a tcp stream, send the data using the nc_send_data2() function and receive data using the nc_receive_data() function.
/// Return the deserialized data as a Result
pub(crate) fn nc_send_receive_data<S: Serialize, D: DeserializeOwned, A: ToSocketAddrs>(data: &S, socket_addr: &A) -> Result<D, NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)?;
    nc_receive_data(&mut tcp_stream)
}
