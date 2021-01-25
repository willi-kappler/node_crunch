use std::time::Duration;

use log::{info, error, debug};

use serde::{Serialize, Deserialize};

use crate::nc_error::{NCError};
use crate::nc_server::{NCServerMessage};
use crate::nc_config::{NCConfiguration};
use crate::nc_node_info::{NodeID};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCNodeMessage {
    Register,
    NeedsData(NodeID),
    HasData(NodeID, Vec<u8>),
    HeartBeat(NodeID),
}

// TODO: Generic trait, U for data in, V for data out
pub trait NCNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Result<Vec<u8>, NCError>;
}

pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    Ok(())
}

/*
pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    let mut event_queue = EventQueue::new();
    let network_sender = event_queue.sender().clone();
    let mut network = Network::new(move |net_event| network_sender.send(NCNodeEvent::InMsg(net_event)));
    let server_endpoint = network.connect_tcp((&config.address as &str, config.port))?;

    // let node_address = network.local_address(server_endpoint.resource_id());

    info!("Connected to server: {}", &config.address);

    event_queue.sender().send_with_timer(NCNodeEvent::Heartbeat, Duration::from_secs(config.heartbeat));
    // TODO: Get hostname
    network.send(server_endpoint, NCNodeMessage::Register("hostname".to_string()));

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
                                network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id));
                            }
                            NCServerMessage::HasData(data) => {
                                info!("Received raw data from server, ready to process...");
                                match nc_node.process_data_from_server(data) {
                                    Ok(data) => {
                                        debug!("Data has been processed successfully, sending to server...");
                                        network.send(server_endpoint, NCNodeMessage::HasData(nc_node_id, data))
                                    }
                                    Err(e) => {
                                        error!("Data from server could not be processed properly: {}", e);
                                        info!("Requesting new data");
                                        // TODO: send messge to server that data could not be processed
                                    }
                                };
                                network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id));
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
                    NetEvent::DeserializationError(_) => {
                        error!("Error while deserializing message");
                    }
                }
            }
            NCNodeEvent::Heartbeat => {
                debug!("Send hearbeat to server");
                network.send(server_endpoint, NCNodeMessage::HeartBeat(nc_node_id));
                event_queue.sender().send_with_timer(NCNodeEvent::Heartbeat, Duration::from_secs(config.heartbeat));
            }
            NCNodeEvent::DelayRequestData => {
                debug!("Delay request data");
                network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id));
            }
        }
    }

    Ok(())
}
*/
