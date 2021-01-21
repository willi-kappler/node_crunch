use std::time::{Duration};

use log::{info, error, debug};

use serde::{Serialize, Deserialize};

use rand::{random};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent, Endpoint};

use crate::nc_error::{NCError};
use crate::nc_node::{NCNodeMessage};
use crate::nc_config::{NCConfiguration};
use crate::nc_node_info::{NCNodeInfo};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCServerMessage {
    AssignNodeID(u64),
    HasData(Vec<u8>),
    Waiting,
    Finished,
    PrepareDataError,
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

// TODO: Generic trait, U for data in, V for data out
pub trait NCServer {
    fn prepare_data_for_node(&mut self, node_id: u64) -> Option<Vec<u8>>; // TODO: Use Result<>
    fn process_data_from_node(&mut self, node_id: u64, data: &Vec<u8>); // TODO: Use Result<>
    fn job_status(&self) -> NCJobStatus;
    fn heartbeat_timeout(&mut self, node_id: u64);
}

pub fn nc_start_server<T: NCServer + Send>(mut nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    let mut event_queue = EventQueue::new();
    let network_sender = event_queue.sender().clone();
    let mut network = NetworkManager::new(move |net_event| network_sender.send(NCServerEvent::InMsg(net_event)));
    network.listen_tcp((config.address, config.port))?;

    event_queue.sender().send(NCServerEvent::CheckHeartbeat);
    event_queue.sender().send(NCServerEvent::CheckJobStatus);

    let mut all_nodes: Vec<NCNodeInfo> = Vec::new();

    loop {
        match event_queue.receive() {
            NCServerEvent::InMsg(msg1) => {
                match msg1 {
                    NetEvent::Message(endpoint, msg2) =>  {
                        let socket_addr = endpoint.addr();

                        match msg2 {
                            NCNodeMessage::Register(hostname) => {
                                info!("Registering new node: {}, {}", hostname, socket_addr);
                                let node_id = get_new_node_id(&all_nodes);
                                let node_info = NCNodeInfo::new(node_id, endpoint, hostname);
                                all_nodes.push(node_info);

                                if network.send(endpoint, NCServerMessage::AssignNodeID(node_id)).is_err() {
                                    error!("Error while sending Register message to node: {}, {}", node_id, socket_addr);
                                }

                            }
                            NCNodeMessage::NeedsData(node_id) => {
                                debug!("Node {} needs data to process", node_id);
                                match nc_server.prepare_data_for_node(node_id) {
                                    Some(data) => {
                                        debug!("Sending data to node: {}", node_id);
                                        if network.send(endpoint, NCServerMessage::HasData(data)).is_err() {
                                            error!("Error while sending HasData message to node: {}, {}", node_id, socket_addr);
                                        }
                                    }
                                    None => {
                                        error!("An error occurred while preparing the data for the node: {}", node_id);
                                        if network.send(endpoint, NCServerMessage::PrepareDataError).is_err() {
                                            error!("Error while sending Error message to node: {}, {}", node_id, socket_addr);
                                        }
                                    }
                                }
                            }
                            NCNodeMessage::HasData(node_id, data) => {
                                debug!("Node {} has processed some data and we received the results", node_id);
                                nc_server.process_data_from_node(node_id, &data);
                            }
                            NCNodeMessage::HeartBeat(node_id) => {
                                update_heartbeat(&mut all_nodes, node_id);
                            }
                        }
                    }
                    NetEvent::AddedEndpoint(endpoint) => {
                        info!("New node connected: {}", endpoint.addr());
                    }
                    NetEvent::RemovedEndpoint(endpoint) => {
                        error!("Node has disconnected: {}", endpoint.addr());
                        remove_node(&mut all_nodes, endpoint);
                    }
                }
            }
            NCServerEvent::CheckHeartbeat => {
                check_heartbeat(&all_nodes, config.heartbeat, &mut nc_server);
                event_queue.sender().send_with_timer(NCServerEvent::CheckHeartbeat, Duration::from_secs(config.heartbeat * 2));
            }
            NCServerEvent::CheckJobStatus => {
                match nc_server.job_status() {
                    NCJobStatus::Unfinished => {
                        debug!("Job is not done yet...");
                        // Nothing to do...
                    }
                    NCJobStatus::Waiting => {
                        debug!("Mostly done, waiting for nodes to finish...");
                        let endpoints = all_nodes.iter().map(|node_info| &node_info.endpoint);
                        // TODO: check Vec of results
                        if let Err(errors) = network.send_all(endpoints, NCServerMessage::Waiting) {
                            for (endpoint, error) in errors.iter() {
                                error!("Error while sending Waiting message to node: {}, error: {}", endpoint.addr(), error);
                            }
                        }
                    }
                    NCJobStatus::Finished => {
                        info!("Job is done!, Will exit now");
                        let endpoints = all_nodes.iter().map(|node_info| &node_info.endpoint);
                        // TODO: check Vec of results
                        if let Err(errors) = network.send_all(endpoints, NCServerMessage::Finished) {
                            for (endpoint, error) in errors.iter() {
                                error!("Error while sending Finished message to node: {}, error: {}", endpoint.addr(), error);
                            }
                        }
                        break;
                    }
                };

                event_queue.sender().send_with_timer(NCServerEvent::CheckJobStatus, Duration::from_secs(config.job_status));
            }
        }
    }

    Ok(())
}

fn get_new_node_id(all_nodes: &Vec<NCNodeInfo>) -> u64 {
    let mut new_id: u64 = random();

    'l1: loop {
        for node_info in all_nodes.iter() {
            if node_info.node_id == new_id {
                new_id = random();
                continue 'l1
            }
        }

        break
    }

    new_id
}

fn update_heartbeat(all_nodes: &mut Vec<NCNodeInfo>, node_id: u64) {
    for node_info in all_nodes.iter_mut() {
        if node_info.node_id == node_id {
            node_info.update_heartbeat();
        }
    }
}

fn remove_node(all_nodes: &mut Vec<NCNodeInfo>, endpoint: Endpoint) {
    all_nodes.retain(|node_info| node_info.endpoint != endpoint);
}

fn check_heartbeat<T: NCServer>(all_nodes: &Vec<NCNodeInfo>, limit: u64, nc_server: &mut T) {
    for node_info in all_nodes.iter() {
        if node_info.heartbeat_invalid(limit) {
            nc_server.heartbeat_timeout(node_info.node_id);
        }
    }
}
