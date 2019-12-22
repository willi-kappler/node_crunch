use tokio::io::{AsyncReadExt, AsyncBufReadExt, AsyncWriteExt};

use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};

use crate::nc_error::{NCError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NCJobStatus {
    Unfinished,
    Waiting,
    Finished,
}

pub async fn nc_send_message<T: AsyncWriteExt + Unpin>(buf_writer: &mut T, message: Vec<u8>) -> Result<(), NCError> {
    buf_writer.write_u64(message.len() as u64).await.map_err(|e| NCError::WriteU64(e))?;
    buf_writer.write(&message).await.map_err(|e| NCError::WriteBuffer(e))?;
    Ok(())
}

pub async fn nc_receive_message<T: AsyncBufReadExt + Unpin>(buf_reader: &mut T) -> Result<(usize, Vec<u8>), NCError> {
    let message_length: u64 = buf_reader.read_u64().await.map_err(|e| NCError::ReadU64(e))?;
    let mut buffer = vec![0; message_length as usize];
    let num_of_bytes_read: usize = buf_reader.read(&mut buffer[..]).await.map_err(|e| NCError::ReadBuffer(e))?;

    Ok((num_of_bytes_read, buffer))
}

pub fn nc_encode_data<T: Serialize>(data: &T) -> Result<Vec<u8>, NCError> {
    serialize(data).map_err(|e| NCError::Serialize(e))
}

pub fn nc_decode_data<'de, T: Deserialize<'de>>(data: &'de Vec<u8>) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}
