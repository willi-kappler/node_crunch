use std::error;

use tokio::net::TcpStream;
use tokio::io::{BufReader, BufWriter, AsyncReadExt, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};

use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};

use crate::nc_error::{NC_Error};
use crate::nc_server::{NC_ServerMessage};
use crate::nc_util::{nc_send_message, nc_receive_message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NC_NodeMessage {
    NodeNeedsData,
    NodeHasData(Vec<u8>),
}

pub trait NC_Node {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Result<Vec<u8>, u8>;
}

pub async fn start_node<T: NC_Node>(mut nc_node: T) -> Result<(), NC_Error> {
    let addr = "127.0.0.1:9000".to_string(); // TODO: read from config file
    let mut quit = false;

    debug!("Connected to server: {}", addr);

    while !quit {
        let mut stream = TcpStream::connect(&addr).await.map_err(|e| NC_Error::TcpConnect(e))?;
        let (reader, writer) = stream.split();
        let mut buf_reader = BufReader::new(reader);
        let mut buf_writer = BufWriter::new(writer);

        let message = nc_encode(NC_NodeMessage::NodeNeedsData)?;

        nc_send_message(&mut buf_writer, message).await?;

        let (num_of_bytes_read, buffer) = nc_receive_message(&mut buf_reader).await?;

        debug!("start_node: number of bytes read: {}", num_of_bytes_read);

        match nc_decode(buffer)? {
            NC_ServerMessage::ServerFinished => quit = true,
            NC_ServerMessage::ServerHasData(data) => {
                let processed_data = nc_node.process_data_from_server(data).map_err(|e| NC_Error::NodeProcess(e))?; // TODO: this may take a lot of time
                let message = nc_encode(NC_NodeMessage::NodeHasData(processed_data))?;

                nc_send_message(&mut buf_writer, message).await?;
            }
        }
    }

    Ok(())
}

fn nc_encode(message: NC_NodeMessage) -> Result<Vec<u8>, NC_Error> {
    serialize(&message).map_err(|e| NC_Error::Serialize(e))
}

fn nc_decode(buffer: Vec<u8>) -> Result<NC_ServerMessage, NC_Error> {
    deserialize(&buffer).map_err(|e| NC_Error::Deserialize(e))
}

