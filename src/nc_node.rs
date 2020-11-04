use std::time::Duration;

use log::{error, debug};

use serde::{Serialize, Deserialize};

use rand::{self, Rng};

use crate::nc_error::{NCError};
use crate::nc_server::{NCServerMessage};
use crate::nc_config::{NCConfiguration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NCNodeMessage {
    Register(NodeInfo),
    NeedsData(u128),
    HasData((u128, Vec<u8>)),
    HeartBeat(u128),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    hostname: String,
    IP: String,

}

pub trait NCNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Vec<u8>;
}

pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    Ok(())
}
