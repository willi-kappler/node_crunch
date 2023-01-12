//! This module contains the NC_Communicator for serializing, deserializing, sending and receiving data.

use std::net::{TcpStream, ToSocketAddrs};
use std::io::{Write, Read};

use serde::{Serialize, de::DeserializeOwned};
use bincode::{deserialize, serialize};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, NewAead};

use crate::nc_config::{NCConfiguration};
use crate::nc_error::NCError;

pub struct NCCommunicator {
    compress: bool,
    encrypt: bool,
    cipher: ChaCha20Poly1305,
    nonce: u64,
}

impl NCCommunicator {
    pub fn new(config: &NCConfiguration) -> Self {
        let key = Key::from_slice(config.key.as_bytes());
        let cipher = ChaCha20Poly1305::new(key);

        Self {
            compress: config.compress,
            encrypt: config.encrypt,
            cipher,
            nonce: 0,
        }
    }

    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, NCError> {
        let bytes: [u8; 8] = self.nonce.to_be_bytes();
        let mut nonce_bytes: Vec<u8> = vec![0, 0, 0, 0];
        nonce_bytes.extend_from_slice(&bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let encrypted_data = self.cipher.encrypt(nonce, data).map_err(|_| NCError::Encrypt)?;
        let mut full_data = Vec::with_capacity(nonce_bytes.len() + encrypted_data.len());
        full_data.extend_from_slice(&nonce_bytes);
        full_data.extend_from_slice(&encrypted_data);
        Ok(full_data)
    }

    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, NCError> {
        let (nonce_bytes, encrypted_data) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher.decrypt(nonce, encrypted_data).map_err(|_| NCError::Encrypt)
    }

    /// Encodes the given data to a [`Vec<u8>`].
    ///
    /// # Errors
    ///
    /// On failure it returns a [`NCError::Serialize`] error which contains the serde serialize error.
    pub(crate) fn nc_encode_data<S: Serialize>(&mut self, data: &S) -> Result<Vec<u8>, NCError> {
        let mut data_out = serialize(data).map_err(|e| NCError::Serialize(e))?;

        if self.encrypt {
            data_out = self.encrypt_data(&data_out)?;
            self.nonce += 1;
        }

        if self.compress {
            data_out = compress_prepend_size(&data_out);
        }

        Ok(data_out)
    }

    /// Decode the given data from a `&[u8]` slice to the type `T`.
    ///
    /// # Errors
    ///
    /// On failure it returns a [`NCError::Deserialize`] error which contains the serde deserialize error.
    pub(crate) fn nc_decode_data<D: DeserializeOwned>(&self, data: &[u8]) -> Result<D, NCError> {
        let mut data_out: Vec<u8> = data.to_vec();

        if self.compress {
            data_out = decompress_size_prepended(&data_out)?;
        }

        if self.encrypt {
            data_out = self.decrypt_data(&data_out)?;
        }

        deserialize(&data_out).map_err(|e| NCError::Deserialize(e))
    }

    /// Open a tcp connection and sends the data using the nc_send_data2() function.
    ///
    /// # Errors
    ///
    /// On failure it returns a [`NCError`].
    pub(crate) fn nc_send_data<S: Serialize, A: ToSocketAddrs>(&mut self, data: &S, socket_addr: &A) -> Result<(), NCError> {
        let mut tcp_stream = TcpStream::connect(socket_addr)?;
        self.nc_send_data2(data, &mut tcp_stream)
    }

    /// Serialize the data and send it to the given Writer (usually a tcp stream).
    ///
    /// # Errors
    ///
    /// On failure it returns a [`NCError`].
    pub(crate) fn nc_send_data2<S: Serialize, W: Write>(&mut self, data: &S, tcp_stream: &mut W) -> Result<(), NCError> {
        let data = self.nc_encode_data(data)?;
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
    pub(crate) fn nc_receive_data<D: DeserializeOwned, R: Read>(&self, tcp_stream: &mut R) -> Result<D, NCError> {
        let mut data_len: [u8; 8] = [0; 8];
        tcp_stream.read_exact(&mut data_len)?;
        let data_len = u64::from_le_bytes(data_len);  // u64 is platform independent, usize is platform dependent

        let mut data: Vec<u8> = vec![0; data_len as usize];
        tcp_stream.read_exact(&mut data)?;

        let result = self.nc_decode_data(&data)?;
        Ok(result)
    }

    /// Open a tcp stream, send the data using the nc_send_data2() function and receive data using the nc_receive_data() function.
    /// Return the deserialized data as a Result.
    ///
    /// # Errors
    ///
    /// On failure it returns a [`NCError`].
    pub(crate) fn nc_send_receive_data<S: Serialize, D: DeserializeOwned, A: ToSocketAddrs>(&mut self, data: &S, socket_addr: &A) -> Result<D, NCError> {
        let mut tcp_stream = TcpStream::connect(socket_addr)?;

        self.nc_send_data2(data, &mut tcp_stream)?;
        self.nc_receive_data(&mut tcp_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let config = NCConfiguration {compress: false, encrypt: false, ..Default::default()};
        let mut nc_communicator = NCCommunicator::new(&config);
        let data1: (String, u32, bool) = ("Hello World!".to_string(), 123456, false);

        let data2 = nc_communicator.nc_encode_data(&data1).unwrap();

        let data3: (String, u32, bool) = nc_communicator.nc_decode_data(&data2).unwrap();

        assert_eq!(data1, data3);
    }

    #[test]
    fn test_encode_decode_compress() {
        let config = NCConfiguration {compress: true, encrypt: false, ..Default::default()};
        let mut nc_communicator = NCCommunicator::new(&config);
        let data1: (String, u32, bool) = ("Hello World!".to_string(), 123456, false);

        let data2 = nc_communicator.nc_encode_data(&data1).unwrap();

        let data3: (String, u32, bool) = nc_communicator.nc_decode_data(&data2).unwrap();

        assert_eq!(data1, data3);
    }

    #[test]
    fn test_encode_decode_encrypt() {
        let config = NCConfiguration {compress: false, encrypt: true, key: "7Fv2YhMzwrQHoXRAirOkB0QQDOjS4qnZ".to_string(), ..Default::default()};
        let mut nc_communicator = NCCommunicator::new(&config);
        let data1: (String, u32, bool) = ("Hello World!".to_string(), 123456, false);

        assert_eq!(nc_communicator.nonce, 0);

        let data2 = nc_communicator.nc_encode_data(&data1).unwrap();

        assert_eq!(nc_communicator.nonce, 1);

        let data3: (String, u32, bool) = nc_communicator.nc_decode_data(&data2).unwrap();

        assert_eq!(nc_communicator.nonce, 1);

        assert_eq!(data1, data3);
    }

    #[test]
    fn test_encode_decode_compress_encrypt() {
        let config = NCConfiguration {compress: true, encrypt: true, key: "VnlUYvqu5S4tNHty0ccA1LAlsgqIXhIs".to_string(), ..Default::default()};
        let mut nc_communicator = NCCommunicator::new(&config);
        let data1: (String, u32, bool) = ("Hello World!".to_string(), 123456, false);

        assert_eq!(nc_communicator.nonce, 0);

        let data2 = nc_communicator.nc_encode_data(&data1).unwrap();

        assert_eq!(nc_communicator.nonce, 1);

        let data3: (String, u32, bool) = nc_communicator.nc_decode_data(&data2).unwrap();

        assert_eq!(nc_communicator.nonce, 1);

        assert_eq!(data1, data3);
    }

    #[test]
    fn test_send_data2() {
        use std::convert::TryInto;

        let config = NCConfiguration {compress: false, encrypt: false, ..Default::default()};
        let mut nc_communicator = NCCommunicator::new(&config);
        let data1: (String, u32, bool) = ("Test send_data2!".to_string(), 121212, false);

        let mut buffer: Vec<u8> = Vec::new();

        nc_communicator.nc_send_data2(&data1, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 37);

        let len_buffer: [u8; 8] = buffer[0..8].try_into().unwrap();
        let data_len = u64::from_le_bytes(len_buffer);

        assert_eq!(data_len, 29);

        let data2: (String, u32, bool) = nc_communicator.nc_decode_data(&buffer[8..]).unwrap();

        assert_eq!(data1, data2);
    }

    #[test]
    fn test_receive_data() {
        let config = NCConfiguration {compress: false, encrypt: false, ..Default::default()};
        let mut nc_communicator = NCCommunicator::new(&config);
        let data1: (String, u32, bool) = ("Test receive_data!".to_string(), 998877, true);
        let mut data2 = nc_communicator.nc_encode_data(&data1).unwrap();

        let data_len = data2.len() as u64;
        let data_len = data_len.to_le_bytes();
        let mut buffer = data_len.to_vec();

        buffer.append(&mut data2);

        let data3: (String, u32, bool) = nc_communicator.nc_receive_data(&mut buffer.as_slice()).unwrap();

        assert_eq!(data1, data3);
    }

    #[test]
    fn test_send_and_receive() {
        let config = NCConfiguration {compress: false, encrypt: false, ..Default::default()};
        let mut nc_communicator = NCCommunicator::new(&config);
        let data1: (String, u32, bool) = ("Test send and then receive data!".to_string(), 550055, true);

        let mut buffer: Vec<u8> = Vec::new();

        nc_communicator.nc_send_data2(&data1, &mut buffer).unwrap();

        let data2: (String, u32, bool) = nc_communicator.nc_receive_data(&mut buffer.as_slice()).unwrap();

        assert_eq!(data1, data2);
    }
}
