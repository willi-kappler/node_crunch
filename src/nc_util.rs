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

    /*
    let data = nc_encode_data(data)?;
    let data_size: [u8; 8] = data.len().to_le_bytes();
    debug!("nc_send_data2(), data_size: {:?}", data_size);
    debug!("nc_send_data2(), buffer length: {}", data.len());
    let length1 = tcp_stream.write(&data_size)?;
    let length2 = tcp_stream.write(&data)?;
    debug!("nc_send_data2(), send {}, {} bytes", length1, length2);
    */

    Ok(())
}

pub(crate) fn nc_receive_data<D: DeserializeOwned>(tcp_stream: &mut TcpStream) -> Result<D, NCError> {
    let result = deserialize_from(tcp_stream)?;
    /*
    let mut data_size: [u8; 8] = [0; 8];
    let length1 = tcp_stream.read(&mut data_size)?;
    debug!("nc_receive_data(), data_size: {:?}", data_size);
    let data_size: usize = usize::from_le_bytes(data_size);
    debug!("nc_receive_data(), buffer length: {}", data_size);
    let mut buffer: Vec<u8> = vec![0; data_size];
    let length2 = tcp_stream.read(&mut buffer)?;
    debug!("nc_receive_data(), read {}, {} bytes", length1, length2);

    let result = nc_decode_data2(&buffer)?;
    */

    Ok(result)
}

pub(crate) fn nc_send_receive_data<S: Serialize, D: DeserializeOwned>(data: &S, socket_addr: &SocketAddr) -> Result<D, NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)?;
    nc_receive_data(&mut tcp_stream)
}
