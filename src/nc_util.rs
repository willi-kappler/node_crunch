use std::net::{SocketAddr, TcpStream};
use std::io::{Write, Read};

use log::{debug};

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bincode::{deserialize, serialize, serialize_into, deserialize_from};

use crate::nc_error::{NCError};

pub fn nc_encode_data<T: Serialize>(data: &T) -> Result<Vec<u8>, NCError> {
    serialize(data).map_err(|e| NCError::Serialize(e))
}

pub fn nc_decode_data<'de, T: Deserialize<'de>>(data: &'de Vec<u8>) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

pub fn nc_decode_data2<T: DeserializeOwned>(data: &Vec<u8>) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

pub(crate) fn nc_send_data<T: Serialize>(data: &T, socket_addr: &SocketAddr) -> Result<(), NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)
}

pub(crate) fn nc_send_data2<S: Serialize>(data: &S, tcp_stream: &mut TcpStream) -> Result<(), NCError> {
    serialize_into(tcp_stream, data)?;
    Ok(())
}

pub(crate) fn nc_receive_data<D: DeserializeOwned>(tcp_stream: &mut TcpStream) -> Result<D, NCError> {
    let result = deserialize_from(tcp_stream)?;
    Ok(result)
}

pub(crate) fn nc_send_receive_data<S: Serialize, D: DeserializeOwned>(data: &S, socket_addr: &SocketAddr) -> Result<D, NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)?;
    nc_receive_data(&mut tcp_stream)
}
