use tokio::net::TcpListener;
use tokio::io::{BufReader, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};


pub enum NodeMessage {
    NodeNeedsData,
    NodeHasData(Vec<u8>),
}

pub trait NC_Node {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Vec<u8>;
}
