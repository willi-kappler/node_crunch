use std::time::{Duration, Instant};
use std::collections::HashMap;

use log::{error, debug};

use serde::{Serialize, Deserialize};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent};

use crate::nc_error::{NCError};
use crate::nc_node::{NCNodeMessage};
use crate::nc_config::{NCConfiguration};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCServerMessage {
    AssignNodeID(u128),
    HasData(Vec<u8>),
    Waiting,
    Finished,
}

#[derive(Debug)]
enum NCServerEvent {
    InMsg(NetEvent<NCNodeMessage>),
    CheckHearbeat,
}

#[derive(Debug)]
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

pub fn nc_start_server<T: 'static + NCServer + Send>(nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    Ok(())
}
