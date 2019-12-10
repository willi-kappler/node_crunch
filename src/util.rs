use tokio::io::{BufReader, BufWriter, AsyncReadExt, AsyncBufReadExt, AsyncWriteExt};

use crate::error::{NCError};

pub async fn send_message<T: AsyncWriteExt + Unpin>(buf_writer: &mut T, message: Vec<u8>) -> Result<(), NCError> {
    buf_writer.write_u64(message.len() as u64).await.map_err(|e| NCError::WriteU64(e))?;
    buf_writer.write(&message).await.map_err(|e| NCError::WriteBuffer(e))?;
    Ok(())
}

pub async fn receive_message() -> Result<usize, NCError> {
    Ok(0)
}
