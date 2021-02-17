//! This module contains helper functions for serializing, deserializing, sending and receiving data.

use std::net::{SocketAddr, TcpStream};

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bincode::{deserialize, serialize, serialize_into, deserialize_from};

use crate::nc_error::NCError;

/// This function encodes data to a Vec<u8>
pub fn nc_encode_data<T: Serialize>(data: &T) -> Result<Vec<u8>, NCError> {
    serialize(data).map_err(|e| NCError::Serialize(e))
}

// TODO: use &[u8]
/// This function dencodes data from a reference to a Vec<u8> to T.
pub fn nc_decode_data<'de, T: Deserialize<'de>>(data: &'de Vec<u8>) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

// TODO use &[u8]
/// This function dencodes data from an owened reference to a Vec<u8> to T.
pub fn nc_decode_data2<T: DeserializeOwned>(data: &Vec<u8>) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

// This function opens a tcp connection and sends the data using the nc_send_data2() function.
pub(crate) fn nc_send_data<T: Serialize>(data: &T, socket_addr: &SocketAddr) -> Result<(), NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)
}

// This function serializes the data and sends it to the given tcp stream.
pub(crate) fn nc_send_data2<S: Serialize>(data: &S, tcp_stream: &mut TcpStream) -> Result<(), NCError> {
    serialize_into(tcp_stream, data)?;
    Ok(())
}

// This function reads data from the given tcp stream and deserializes it.
pub(crate) fn nc_receive_data<D: DeserializeOwned>(tcp_stream: &mut TcpStream) -> Result<D, NCError> {
    let result = deserialize_from(tcp_stream)?;
    Ok(result)
}

// This function opens a tcp stream, sends the data using the nc_send_data2() function and received data using the nc_receive_data() function.
pub(crate) fn nc_send_receive_data<S: Serialize, D: DeserializeOwned>(data: &S, socket_addr: &SocketAddr) -> Result<D, NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)?;
    nc_receive_data(&mut tcp_stream)
}
