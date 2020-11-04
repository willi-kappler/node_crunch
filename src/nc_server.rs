use std::time::{Duration, Instant};
use std::collections::HashMap;

use log::{info, error, debug};

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
    Error(u8), // TODO: use proper error enum
}

#[derive(Debug)]
enum NCServerEvent {
    InMsg(NetEvent<NCNodeMessage>),
    CheckHeartbeat,
    CheckJobStatus,
}

#[derive(Debug)]
pub enum NCJobStatus {
    Unfinished,
    Waiting,
    Finished,
}

pub trait NCServer {
    fn prepare_data_for_node(&mut self, node_id: u128) -> Option<Vec<u8>>; // TODO: Use Result<>
    fn process_data_from_node(&mut self, node_id: u128, data: &Vec<u8>); // TODO: Use Result<>
    fn job_status(&self) -> NCJobStatus;
    fn heartbeat_timeout(&mut self, node_id: u128);
}

pub fn nc_start_server<T: NCServer + Send>(mut nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    let mut event_queue = EventQueue::new();
    let network_sender = event_queue.sender().clone();
    let mut network = NetworkManager::new(move |net_event| network_sender.send(NCServerEvent::InMsg(net_event)));
    network.listen_tcp(config.address)?; // TODO: Port is missing!

    event_queue.sender().send(NCServerEvent::CheckHeartbeat);

    let mut all_node_ids: Vec<u128> = Vec::new();

    loop {
        match event_queue.receive() {
            NCServerEvent::InMsg(msg1) => {
                match msg1 {
                    NetEvent::Message(endpoint, msg2) =>  {
                        match msg2 {
                            NCNodeMessage::Register(node_info) => {
                                info!("Registering new node: {}, {}", node_info.hostname, node_info.IP);
                            }
                            NCNodeMessage::NeedsData(node_id) => {
                                debug!("Node {} needs data to process", node_id);
                                match nc_server.prepare_data_for_node(node_id) {
                                    Some(data) => {
                                        debug!("Sending data to node: {}", node_id);
                                        network.send(endpoint, NCServerMessage::HasData(data));
                                    }
                                    None => {
                                        error!("An error occurred while preparing the data for the node: {}", node_id);
                                        network.send(endpoint, NCServerMessage::Error(1));
                                    }
                                }
                            }
                            NCNodeMessage::HasData(node_id, data) => {
                                debug!("Node {} has processed some data and we received the results", node_id);
                                nc_server.process_data_from_node(node_id, &data);
                            }
                            NCNodeMessage::HeartBeat(node_id) => {
                                // TODO: save new time stamp for node_id
                            }
                        }
                    }
                    NetEvent::AddedEndpoint(endpoint) => {
                        info!("New client connected: {}", endpoint.addr());
                        // TODO: Add new node in node list
                    }
                    NetEvent::RemovedEndpoint(endpoint) => {
                        error!("Client has disconnected: {}", endpoint.addr());
                        // TODO: remove node from node list
                    }
                }
            }
            NCServerEvent::CheckHeartbeat => {
                // TODO: Go through list of all clients and check if each heartbeat was sent within the limits
                event_queue.sender().send_with_timer(NCServerEvent::CheckHeartbeat, Duration::from_secs(config.heartbeat * 2));
            }
            NCServerEvent::CheckJobStatus => {
                match nc_server.job_status() {
                    NCJobStatus::Unfinished => {
                        info!("Job is not done yet...");
                        // Nothing to do...
                    }
                    NCJobStatus::Waiting => {
                        info!("Mostly done, waiting for nodes to finish...");
                        // network.send_all(endpoints, NCServerMessage::Waiting);
                    }
                    NCJobStatus::Finished => {
                        info!("Job is done!, Will exit now");
                        // network.send_all(endpoints, NCServerMessage::Finished);
                        break;
                    }
                };

                event_queue.sender().send_with_timer(NCServerEvent::CheckJobStatus, Duration::from_secs(config.job_status));
            }
        }
    }

    Ok(())
}
