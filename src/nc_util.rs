//! This module contains helper functions for serializing, deserializing, sending and receiving data.

use std::net::{TcpStream, ToSocketAddrs};
use std::io::{Write, Read};

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bincode::{deserialize, serialize};

use crate::nc_error::NCError;

/// Encodes the given data to a [`Vec<u8>`].
///
/// # Errors
///
/// On failure it returns a [`NCError::Serialize`] error which contains the serde serialize error.
pub fn nc_encode_data<S: Serialize>(data: &S) -> Result<Vec<u8>, NCError> {
    serialize(data).map_err(|e| NCError::Serialize(e))
}

/// Decode the given data from a `&[u8]` slice to the type `T`.
///
/// # Errors
///
/// On failure it returns a [`NCError::Deserialize`] error which contains the serde deserialize error.
pub fn nc_decode_data<'de, D: Deserialize<'de>>(data: &'de [u8]) -> Result<D, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

/// Decode the data from an owned reference `&[u8]` to the type `T`.
///
/// # Errors
///
/// On failure it returns a [`NCError::Deserialize`] error which contains the serde deserialize error.
pub fn nc_decode_data2<D: DeserializeOwned>(data: &[u8]) -> Result<D, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}

/// Open a tcp connection and sends the data using the nc_send_data2() function.
///
/// # Errors
///
/// On failure it returns a [`NCError`].
pub(crate) fn nc_send_data<S: Serialize, A: ToSocketAddrs>(data: &S, socket_addr: &A) -> Result<(), NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    nc_send_data2(data, &mut tcp_stream)
}

/// Serialize the data and send it to the given Writer (usually a tcp stream).
///
/// # Errors
///
/// On failure it returns a [`NCError`].
pub(crate) fn nc_send_data2<S: Serialize, W: Write>(data: &S, tcp_stream: &mut W) -> Result<(), NCError> {
    let data = nc_encode_data(data)?;
    let data_len = data.len() as u64; // u64 is platform independent, usize is platform dependent

    tcp_stream.write_all(&data_len.to_le_bytes())?;
    tcp_stream.write_all(&data)?;
    tcp_stream.flush().map_err(|e| e.into())
}

/// Read data from the given Reader (usually a tcp stream) and deserialize it.
/// Return the deserialized data as a Result.
///
/// # Errors
///
/// On failure it returns a [`NCError`].
pub(crate) fn nc_receive_data<D: DeserializeOwned, R: Read>(tcp_stream: &mut R) -> Result<D, NCError> {
    let mut data_len: [u8; 8] = [0; 8];
    tcp_stream.read_exact(&mut data_len)?;
    let data_len = u64::from_le_bytes(data_len);  // u64 is platform independent, usize is platform dependent

    let mut data: Vec<u8> = vec![0; data_len as usize];
    tcp_stream.read_exact(&mut data)?;

    let result = nc_decode_data(&data)?;
    Ok(result)
}

/// Open a tcp stream, send the data using the nc_send_data2() function and receive data using the nc_receive_data() function.
/// Return the deserialized data as a Result.
///
/// # Errors
///
/// On failure it returns a [`NCError`].
pub(crate) fn nc_send_receive_data<S: Serialize, D: DeserializeOwned, A: ToSocketAddrs>(data: &S, socket_addr: &A) -> Result<D, NCError> {
    let mut tcp_stream = TcpStream::connect(socket_addr)?;

    nc_send_data2(data, &mut tcp_stream)?;
    nc_receive_data(&mut tcp_stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let data1: (String, u32, bool) = ("Hello World!".to_string(), 123456, false);

        let data2 = nc_encode_data(&data1).unwrap();

        let data3: (String, u32, bool) = nc_decode_data(&data2).unwrap();

        assert_eq!(data1, data3);
    }

    #[test]
    fn test_send_data2() {
        use std::convert::TryInto;

        let data1: (String, u32, bool) = ("Test send_data2!".to_string(), 121212, false);

        let mut buffer: Vec<u8> = Vec::new();

        nc_send_data2(&data1, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 37);

        let len_buffer: [u8; 8] = buffer[0..8].try_into().unwrap();
        let data_len = u64::from_le_bytes(len_buffer);

        assert_eq!(data_len, 29);

        let data2: (String, u32, bool) = nc_decode_data(&buffer[8..]).unwrap();

        assert_eq!(data1, data2);
    }

    #[test]
    fn test_receive_data() {
        let data1: (String, u32, bool) = ("Test receive_data!".to_string(), 998877, true);
        let mut data2 = nc_encode_data(&data1).unwrap();

        let data_len = data2.len() as u64;
        let data_len = data_len.to_le_bytes();
        let mut buffer = data_len.to_vec();

        buffer.append(&mut data2);

        let data3: (String, u32, bool) = nc_receive_data(&mut buffer.as_slice()).unwrap();

        assert_eq!(data1, data3);
    }

    #[test]
    fn test_send_and_receive() {
        let data1: (String, u32, bool) = ("Test send and then receive data!".to_string(), 550055, true);

        let mut buffer: Vec<u8> = Vec::new();

        nc_send_data2(&data1, &mut buffer).unwrap();

        let data2: (String, u32, bool) = nc_receive_data(&mut buffer.as_slice()).unwrap();

        assert_eq!(data1, data2);
    }
}
