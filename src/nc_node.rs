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
    Register(NCNodeInfo),
    NeedsData(u128),
    HasData(u128, Vec<u8>),
    HeartBeat(u128),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NCNodeInfo {
    hostname: String,
    IP: String,

}

#[derive(Debug)]
enum NCNodeEvent {
    InMsg(NetEvent<NCServerMessage>),
    Heartbeat,
    DelayRequestData,
}

pub trait NCNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Option<Vec<u8>>;
}

pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    let mut event_queue = EventQueue::new();
    let network_sender = event_queue.sender().clone();
    let mut network = NetworkManager::new(move |net_event| network_sender.send(NCNodeEvent::InMsg(net_event)));

    let server_endpoint = network.connect_tcp(&config.address)?;

    let node_address = network.local_address(server_endpoint.resource_id());

    info!("Connected to server: {}", &config.address);

    // TODO: Get hostname and IP address of node
    let node_info = NCNodeInfo{ hostname: "TODO".to_string(), IP: "TODO".to_string() };
    event_queue.sender().send(NCNodeEvent::Heartbeat);
    network.send(server_endpoint, NCNodeMessage::Register(node_info))?;

    let mut nc_node_id: u128 = 0;

    loop {
        match event_queue.receive() {
            NCNodeEvent::InMsg(msg) => {
                match msg {
                    NetEvent::Message(_, server_msg) => {
                        match server_msg {
                            NCServerMessage::AssignNodeID(id) => {
                                nc_node_id = id;
                                network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id))?;
                            }
                            NCServerMessage::HasData(data) => {
                                match nc_node.process_data_from_server(data) {
                                    Some(data) => {
                                        network.send(server_endpoint, NCNodeMessage::HasData(nc_node_id, data))?;
                                    }
                                    None => {
                                        error!("Data from server could not be processed properly, requesting new data");
                                    }
                                };
                                network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id))?;

                            }
                            NCServerMessage::Waiting => {
                                // Server is still waiting for other nodes to complete but
                                // it doesn't have any work for us to do now.
                                event_queue.sender().send_with_timer(NCNodeEvent::DelayRequestData, Duration::from_secs(config.delay_request_data));
                            }
                            NCServerMessage::Finished => {
                                info!("Server is done, exit now");
                                break;
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
                network.send(server_endpoint, NCNodeMessage::HeartBeat(nc_node_id))?;
                event_queue.sender().send_with_timer(NCNodeEvent::Heartbeat, Duration::from_secs(config.heartbeat));
            }
            NCNodeEvent::DelayRequestData => {
                debug!("Delay request data");
                network.send(server_endpoint, NCNodeMessage::NeedsData(nc_node_id))?;
            }
        }
    }

    Ok(())
}
