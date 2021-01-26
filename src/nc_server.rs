use std::time::{Duration};
use std::sync::{Arc, Mutex};
use std::{thread, time};

use log::{info, error, debug};

use serde::{Serialize, Deserialize};

use crate::{NCNode, nc_error::{NCError}};
use crate::nc_node::{NCNodeMessage};
use crate::nc_config::{NCConfiguration};
use crate::nc_node_info::{NCNodeInfo, NodeID};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCServerMessage {
    InitialData(NodeID, Vec<u8>),
    HasData(Vec<u8>),
    HeartBeatOK(NCJobStatus),
    Waiting,
    Finished,
    PrepareDataError,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NCJobStatus {
    Unfinished,
    Waiting,
    Finished,
}

// TODO: Generic trait, U for data in, V for data out
pub trait NCServer {
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<Option<Vec<u8>>, NCError>;
    fn process_data_from_node(&mut self, node_id: NodeID, data: &Vec<u8>) -> Result<(), NCError>;
    fn job_status(&self) -> NCJobStatus;
    fn heartbeat_timeout(&mut self, node_id: NodeID);
    fn finish_job(&mut self);
}

pub fn nc_start_server<T: 'static + NCServer + Send>(mut nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    let nc_server = Arc::new(Mutex::new(nc_server));
    let mut node_list = Arc::new(Mutex::new(Vec::<NCNodeInfo>::new()));

    let heartbeat_handle = start_heartbeat_check(2 * config.heartbeat, node_list.clone(), nc_server.clone());

    start_main_loop(node_list, nc_server)?;

    heartbeat_handle.join();

    Ok(())
}

fn start_heartbeat_check<T: 'static + NCServer + Send>(heartbeat_duration: u64, node_list: Arc<Mutex<Vec<NCNodeInfo>>>, nc_server: Arc<Mutex<T>>) -> thread::JoinHandle<()> {
    debug!("Start heartbeat check");

    thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::from_secs(heartbeat_duration));

            match node_list.lock() {
                Ok(node_list) => {
                    for node in node_list.iter() {
                        if node.heartbeat_invalid(heartbeat_duration) {
                            match nc_server.lock() {
                                Ok(mut nc_server) => {
                                    nc_server.heartbeat_timeout(node.node_id);
                                }
                                Err(e) => {
                                    error!("NC server lock error while checking heartbeats: {}", e);
                                }
                            }
                        }
                    }        
                }
                Err(e) => {
                    error!("Node list lock error while checking hearbeats: {}", e);
                }
            }
        }
    })
}

fn start_main_loop<T: NCServer>(node_list: Arc<Mutex<Vec<NCNodeInfo>>>, nc_server: Arc<Mutex<T>>) -> Result<(), NCError> {
    Ok(())
}

/*
pub fn nc_start_server<T: NCServer + Send>(mut nc_server: T, config: NCConfiguration) -> Result<T, NCError> {
    let mut event_queue = EventQueue::new();
    let network_sender = event_queue.sender().clone();
    let mut network = Network::new(move |net_event| network_sender.send(NCServerEvent::InMsg(net_event)));
    network.listen_tcp((config.address, config.port))?;

    event_queue.sender().send_with_timer(NCServerEvent::CheckHeartbeat, Duration::from_secs(config.heartbeat * 2));
    event_queue.sender().send_with_timer(NCServerEvent::CheckJobStatus, Duration::from_secs(config.job_status));

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

                                network.send(endpoint, NCServerMessage::AssignNodeID(node_id));

                            }
                            NCNodeMessage::NeedsData(node_id) => {
                                debug!("Node {} needs data to process", node_id);
                                match nc_server.prepare_data_for_node(node_id) {
                                    Ok(Some(data)) => {
                                        debug!("Sending data to node: {}", node_id);
                                        network.send(endpoint, NCServerMessage::HasData(data));
                                    }
                                    Ok(None) => {
                                        debug!("No more data to send, waiting for other nodes to finish");
                                    }
                                    Err(e) => {
                                        error!("An error occurred while preparing the data for the node: {}, error: {}", node_id, e);
                                        network.send(endpoint, NCServerMessage::PrepareDataError);
                                    }
                                }
                            }
                            NCNodeMessage::HasData(node_id, data) => {
                                debug!("Node {} has processed some data and we received the results", node_id);
                                match nc_server.process_data_from_node(node_id, &data) {
                                    Ok(_) => {
                                        debug!("Processing data from node done, no errors.");
                                    }
                                    Err(e) => {
                                        error!("Could not process data from node, error: {}", e);
                                    }
                                }
                            }
                            NCNodeMessage::HeartBeat(node_id) => {
                                debug!("Got hearbeat from node: {}", node_id);
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
                    NetEvent::DeserializationError(_) => {
                        error!("Error while deserializing message");
                    }
                }
            }
            NCServerEvent::CheckHeartbeat => {
                debug!("Check heartbeat for all nodes (limit: {})", config.heartbeat);
                check_heartbeat(&all_nodes, config.heartbeat, &mut nc_server);
                event_queue.sender().send_with_timer(NCServerEvent::CheckHeartbeat, Duration::from_secs(config.heartbeat * 2));
            }
            NCServerEvent::CheckJobStatus => {
                match nc_server.job_status() {
                    NCJobStatus::Unfinished => {
                        debug!("Job is not done yet...");
                        // Nothing else to do...
                    }
                    NCJobStatus::Waiting => {
                        debug!("Mostly done, waiting for nodes to finish...");
                        let endpoints = all_nodes.iter().map(|node_info| &node_info.endpoint);
                        network.send_all(endpoints, NCServerMessage::Waiting);
                    }
                    NCJobStatus::Finished => {
                        info!("Job is done!, Will exit now");
                        let endpoints = all_nodes.iter().map(|node_info| &node_info.endpoint);
                        network.send_all(endpoints, NCServerMessage::Finished);
                        break;
                    }
                };

                event_queue.sender().send_with_timer(NCServerEvent::CheckJobStatus, Duration::from_secs(config.job_status));
            }
        }
    }

    Ok(nc_server)
}
*/

fn get_new_node_id(all_nodes: &Vec<NCNodeInfo>) -> NodeID {
    let mut new_id: NodeID = NodeID::random();

    'l1: loop {
        for node_info in all_nodes.iter() {
            if node_info.node_id == new_id {
                new_id = NodeID::random();
                continue 'l1
            }
        }

        break
    }

    new_id
}

fn update_heartbeat(all_nodes: &mut Vec<NCNodeInfo>, node_id: NodeID) {
    for node_info in all_nodes.iter_mut() {
        if node_info.node_id == node_id {
            node_info.update_heartbeat();
        }
    }
}

fn check_heartbeat<T: NCServer>(all_nodes: &Vec<NCNodeInfo>, limit: u64, nc_server: &mut T) {
    for node_info in all_nodes.iter() {
        if node_info.heartbeat_invalid(limit) {
            nc_server.heartbeat_timeout(node_info.node_id);
        }
    }
}
