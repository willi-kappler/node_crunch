use std::time::Duration;

use log::{info, error, debug};

use serde::{Serialize, Deserialize};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent};

use crate::nc_error::{NCError};
use crate::nc_server::{NCServerMessage};
use crate::nc_config::{NCConfiguration};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCNodeMessage {
    Register(String),
    NeedsData(u64),
    HasData(u64, Vec<u8>),
    HeartBeat(u64),
}

#[derive(Debug)]
enum NCNodeEvent {
    InMsg(NetEvent<NCServerMessage>),
    Heartbeat,
    DelayRequestData,
}

// TODO: Generic trait, U for data in, V for data out
pub trait NCNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Option<Vec<u8>>; // TODO: Use Result<>
}

pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    let mut event_queue = EventQueue::new();
    let network_sender = event_queue.sender().clone();
    let mut network = NetworkManager::new(move |net_event| network_sender.send(NCNodeEvent::InMsg(net_event)));
    let server_endpoint = network.connect_tcp((&config.address as &str, config.port))?;

    let node_address = network.local_address(server_endpoint.resource_id());

    info!("Connected to server: {}", &config.address);

    event_queue.sender().send(NCNodeEvent::Heartbeat);
    // TODO: Get hostname
    network.send(server_endpoint, NCNodeMessage::Register("hostname".to_string()))?;

    let mut nc_node_id: u64 = 0;

    loop {
        match event_queue.receive() {
            NCNodeEvent::InMsg(msg) => {
                match msg {
                    NetEvent::Message(_, server_msg) => {
                        match server_msg {
                            NCServerMessage::AssignNodeID(id) => {
                                info!("New node id assigned: {}", id);
                                nc_node_id = id;
                                if network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id)).is_err() {
                                    error!("Could not send NeedsData message to server");
                                }
                            }
                            NCServerMessage::HasData(data) => {
                                info!("Received raw data from server, ready to process...");
                                match nc_node.process_data_from_server(data) {
                                    Some(data) => {
                                        debug!("Data has been processed successfully, sending to server...");
                                        if network.send(server_endpoint, NCNodeMessage::HasData(nc_node_id, data)).is_err() {
                                            error!("Could not send HasData message to server");
                                        }
                                    }
                                    None => {
                                        error!("Data from server could not be processed properly, requesting new data");
                                        // TODO: send messge to server that data could not be processed
                                    }
                                };
                                if network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id)).is_err() {
                                    error!("Could not send NeedsData message to server");
                                }

                            }
                            NCServerMessage::Waiting => {
                                // Server is still waiting for other nodes to complete but
                                // it doesn't have any work for us to do now.
                                info!("Server is waiting for other nodes to finish...");
                                debug!("Will retry in {} seconds", config.delay_request_data);
                                event_queue.sender().send_with_timer(NCNodeEvent::DelayRequestData, Duration::from_secs(config.delay_request_data));
                            }
                            NCServerMessage::Finished => {
                                info!("Server is done, exit now");
                                break;
                            }
                            NCServerMessage::PrepareDataError => {
                                error!("Server has sent a PrepareDataError, will retry in {} seconds", config.delay_request_data);
                                event_queue.sender().send_with_timer(NCNodeEvent::DelayRequestData, Duration::from_secs(config.delay_request_data));
                            }
                        }
                    }
                    NetEvent::RemovedEndpoint(_) => {
                        // TODO: Try reconnect
                        error!("Connection to server lost. Exit now");
                        break;
                    }
                    NetEvent::AddedEndpoint(_) => {
                        error!("Received 'AddedEndpoint' message. This should not happen!");
                    }
                }
            }
            NCNodeEvent::Heartbeat => {
                debug!("Send hearbeat to server");
                if network.send(server_endpoint, NCNodeMessage::HeartBeat(nc_node_id)).is_err() {
                    error!("Could not send HearBeat message to server");
                }
                event_queue.sender().send_with_timer(NCNodeEvent::Heartbeat, Duration::from_secs(config.heartbeat));
            }
            NCNodeEvent::DelayRequestData => {
                debug!("Delay request data");
                if network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id)).is_err() {
                    error!("Could not send delayed NeedsData message to server");
                }
            }
        }
    }

    Ok(())
}
