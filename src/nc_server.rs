use std::time::{Duration, Instant};
use std::collections::HashMap;

use log::{error, debug};

use serde::{Serialize, Deserialize};

use crate::nc_error::{NCError};
use crate::nc_node::{NCNodeMessage};
use crate::nc_config::{NCConfiguration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NCServerMessage {
    HasData(Vec<u8>),
    Waiting,
    Finished,
    HeartBeatMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NCJobStatus {
    Unfinished,
    Waiting,
    Finished,
}

pub trait NCServer {
    fn prepare_data_for_node(&mut self, node_id: u128) -> Vec<u8>;
    fn process_data_from_node(&mut self, node_id: u128, data: &Vec<u8>);
    fn job_status(&self) -> NCJobStatus;
    fn heartbeat_timeout(&mut self, node_id: u128);
}

pub async fn nc_start_server<T: 'static + NCServer + Send>(nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    Ok(())
}
