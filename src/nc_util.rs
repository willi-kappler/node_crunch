use std::net::{SocketAddr, TcpStream};
use std::io::{Write, Read};

use log::{debug};

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bincode::{deserialize, serialize};

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

pub fn nc_send_data<T: Serialize>(data: &T, socket_addr: &SocketAddr) -> Result<(), NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    let data = nc_encode_data(data)?;
    let length = tcp_stream.write(&data)?;
    debug!("nc_send_data send {} bytes", length);

    Ok(())
}

pub fn nc_send_receive_data<S: Serialize, D: DeserializeOwned>(data: &S, socket_addr: &SocketAddr) -> Result<D, NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    let data = nc_encode_data(data)?;
    let length = tcp_stream.write(&data)?;
    debug!("nc_send_receive_data send {} bytes", length);

    let mut buffer: Vec<u8> = Vec::new();
    let length = tcp_stream.read_to_end(&mut buffer)?;
    debug!("nc_send_receive_data read {} bytes", length);

    let result = nc_decode_data2(&buffer)?;
    Ok(result)
}
