use tokio::net::TcpListener;
use tokio::io::{BufReader, BufWriter, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};

use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};

use crate::error::{NCError};
use crate::nc_server::{ServerMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeMessage {
    NodeNeedsData,
    NodeHasData(Vec<u8>),
}

pub trait NC_Node {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Vec<u8>;
}



fn encode(message: NodeMessage) -> Result<Vec<u8>, NCError> {
    serialize(&message).map_err(|e| NCError::Serialize(e))
}

fn decode(buffer: Vec<u8>) -> Result<ServerMessage, NCError> {
    deserialize(&buffer).map_err(|e| NCError::Deserialize(e))
}

