use tokio::net::TcpStream;
use tokio::io::{BufReader, BufWriter, AsyncReadExt, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};

use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};

use crate::error::{NCError};
use crate::nc_server::{ServerMessage};
use crate::util::{send_message, receive_message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeMessage {
    NodeNeedsData,
    NodeHasData(Vec<u8>),
}

pub trait NC_Node {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Vec<u8>; // TODO: this may fail
}

pub async fn start_node<T: NC_Node>(mut nc_node: T) -> Result<(), NCError> {
    let addr = "127.0.0.1:9000".to_string(); // TODO: read from config file
    let mut quit = false;

    debug!("Connected to server: {}", addr);

    while !quit {
        let mut stream = TcpStream::connect(&addr).await.map_err(|e| NCError::TcpConnect(e))?;
        let (reader, writer) = stream.split();
        let mut buf_reader = BufReader::new(reader);
        let mut buf_writer = BufWriter::new(writer);

        let message = encode(NodeMessage::NodeNeedsData)?;

        send_message(&mut buf_writer, message).await?;

        let message_length: u64 = buf_reader.read_u64().await.map_err(|e| NCError::ReadU64(e))?;
        let mut buffer = vec![0; message_length as usize];
        let num_of_bytes_read: usize = buf_reader.read(&mut buffer[..]).await.map_err(|e| NCError::ReadBuffer(e))?;

        debug!("start_node: message length: {}, number of bytes read: {}", message_length, num_of_bytes_read);

        match decode(buffer)? {
            ServerMessage::ServerFinished => quit = true,
            ServerMessage::ServerHasData(data) => {
                let processed_data = nc_node.process_data_from_server(data); // TODO: this may take a lot of time
                let message = encode(NodeMessage::NodeHasData(processed_data))?;

                buf_writer.write_u64(message.len() as u64).await.map_err(|e| NCError::WriteU64(e))?;
                buf_writer.write(&message).await.map_err(|e| NCError::WriteBuffer(e))?;
            }
        }
    }

    Ok(())
}

fn encode(message: NodeMessage) -> Result<Vec<u8>, NCError> {
    serialize(&message).map_err(|e| NCError::Serialize(e))
}

fn decode(buffer: Vec<u8>) -> Result<ServerMessage, NCError> {
    deserialize(&buffer).map_err(|e| NCError::Deserialize(e))
}

