use std::time::Duration;

use tokio::net::TcpStream;
use tokio::io::{BufReader, BufWriter};
use tokio::time::delay_for;
use tokio::task;

use log::{error, debug};

use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};

use rand::{self, Rng};

use crate::nc_error::{NC_Error};
use crate::nc_server::{NC_ServerMessage};
use crate::nc_util::{nc_send_message, nc_receive_message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NC_NodeMessage {
    NodeNeedsData(u128),
    NodeHasData((u128, Vec<u8>)),
}

pub trait NC_Node {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Result<Vec<u8>, String>;
}

pub async fn start_node<T: NC_Node>(mut nc_node: T) {
    // TODO: read from config file
    let addr = "127.0.0.1:9000".to_string();

    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes[..]);
    let node_id: u128 = u128::from_le_bytes(bytes);

    debug!("Current node id: {}", node_id);

    loop {
        match node_worker(&mut nc_node, &addr, node_id).await {
            Ok(quit) => {
                if quit {
                    debug!("Job is finished, exit loop");
                    break
                }
            }
            Err(e) => {
                error!("An error occurred: {}", e);
                debug!("Retry in 10 seconds");
                delay_for(Duration::from_secs(10)).await;
            }
        }
    }
}

pub async fn node_worker<T: NC_Node>(nc_node: &mut T, addr: &str, node_id: u128) -> Result<bool, NC_Error> {
    let mut quit = false;

    debug!("Connecting to server: {}", addr);
    let mut stream = TcpStream::connect(&addr).await.map_err(|e| NC_Error::TcpConnect(e))?;
    let (reader, writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut buf_writer = BufWriter::new(writer);

    debug!("Encoding message NodeNeedsData");
    let message = nc_encode(NC_NodeMessage::NodeNeedsData(node_id))?;

    debug!("Sending message to server");
    nc_send_message(&mut buf_writer, message).await?;

    debug!("Receiving message from server");
    let (num_of_bytes_read, buffer) = nc_receive_message(&mut buf_reader).await?;

    debug!("Number of bytes read: {}", num_of_bytes_read);
    debug!("Decoding message");
    match nc_decode(buffer)? {
        NC_ServerMessage::ServerFinished => {
            debug!("Received ServerFinished");
            quit = true;
        }
        NC_ServerMessage::ServerHasData(data) => {
            debug!("Received ServerHasData");
            debug!("Processing data...");

            let processed_data = task::block_in_place(move || {
                nc_node.process_data_from_server(data).map_err(|e| NC_Error::NodeProcess(e))
            })?;

            debug!("Encoding message NodeHasData");
            let message = nc_encode(NC_NodeMessage::NodeHasData((node_id, processed_data)))?;

            debug!("Send message back to server");
            nc_send_message(&mut buf_writer, message).await?;
        }
    }

    Ok(quit)
}

fn nc_encode(message: NC_NodeMessage) -> Result<Vec<u8>, NC_Error> {
    serialize(&message).map_err(|e| NC_Error::Serialize(e))
}

fn nc_decode(buffer: Vec<u8>) -> Result<NC_ServerMessage, NC_Error> {
    deserialize(&buffer).map_err(|e| NC_Error::Deserialize(e))
}
