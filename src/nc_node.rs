use std::time::Duration;
use std::net::TcpStream;
use std::net::{IpAddr, SocketAddr};
use std::{thread, time};

use log::{info, error, debug};

use serde::{Serialize, Deserialize};

use crate::{nc_error::{NCError}, nc_send_data};
use crate::nc_server::{NCServerMessage, NCJobStatus};
use crate::nc_config::{NCConfiguration};
use crate::nc_node_info::{NodeID};
use crate::nc_util::{nc_send_receive_data};

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
    fn set_initial_data(&mut self, node_id: NodeID, initial_data: Vec<u8>) -> Result<(), NCError>;
}

pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    let ip_addr: IpAddr = config.address.parse()?;
    let socket_addr = SocketAddr::new(ip_addr, config.port);

    let (node_id, initial_data) = get_initial_data(&socket_addr)?;

    nc_node.set_initial_data(node_id, initial_data)?;

    let heartbeat_duration = config.heartbeat;
    let heartbeat_thread = start_hearbeat(node_id, socket_addr, heartbeat_duration);

    start_main_loop(nc_node, socket_addr, config, node_id);

    heartbeat_thread.join().map_err(|_| NCError::ThreadJoin)?;
    Ok(())
}

fn get_initial_data(socket_addr: &SocketAddr) -> Result<(NodeID, Vec<u8>), NCError> {
    debug!("Get initial data");

    let initial_data = nc_send_receive_data(&NCNodeMessage::Register, socket_addr)?;

    match initial_data {
        NCServerMessage::InitialData(node_id, initial_data) => {
            Ok((node_id, initial_data))
        }
        msg => {
            error!("NCServerMessage missmatch, expected: InitialData, got: {:?}", msg);
            Err(NCError::ServerMsgMismatch)
        }
    }
}

fn start_hearbeat(node_id: NodeID, socket_addr: SocketAddr, heartbeat_duration: u64) -> thread::JoinHandle<()> {
    debug!("Start heartbeat thread, node_id: {:?}, heartbeat_duration: {}", node_id, heartbeat_duration);

    thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::from_secs(heartbeat_duration));

            match nc_send_receive_data(&NCNodeMessage::HeartBeat(node_id), &socket_addr) {
                Ok(NCServerMessage::HeartBeatOK(NCJobStatus::Finished)) => {
                    break
                }
                Ok(NCServerMessage::HeartBeatOK(_)) => {
                    continue
                }
                Ok(msg) => {
                    error!("NCServerMessage missmatch, expected: InitialData, got: {:?}", msg);
                }
                Err(e) => {
                    error!("Error while sending hearbeat to server: {}", e);
                }
            }
        }
    })
}

fn start_main_loop<T: NCNode>(mut nc_node: T, socket_addr: SocketAddr, config: NCConfiguration, node_id: NodeID) {
    loop {
        debug!("Ask server for new data");

        match nc_send_receive_data(&NCNodeMessage::NeedsData(node_id), &socket_addr) {
            Ok(NCServerMessage::HasData(data)) => {
                debug!("New data received from server");

                match nc_node.process_data_from_server(data) {
                    Ok(result) => {
                        match nc_send_data(&result, &socket_addr) {
                            Ok(_) => {
                                debug!("Processed data has been send sucessfully to server");
                            }
                            Err(e) => {
                                error!("Error while sending processed data to server: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error while processing data from server: {}", e);
                    }
                }
            }
            Ok(NCServerMessage::Waiting) => {
                // The node will not exit here since the job is not 100% done.
                // This just means that all the remaining work has already
                // been distributed among all nodes.
                // One of the nodes can still crash and thus free nodes have to ask the server for more work
                // from time to time (delay_request_data).

                debug!("Waiting for other nodes to finish...");
                debug!("Will try again in {} seconds (delay_request_data)", config.delay_request_data);
                thread::sleep(time::Duration::from_secs(config.delay_request_data));
            }
            Ok(NCServerMessage::Finished) => {
                debug!("Job is done, exit now.");
                break
            }
            Ok(NCServerMessage::PrepareDataError) => {
                error!("Error while preparing data for node.");
                debug!("Will try again in {} seconds (delay_request_data)", config.delay_request_data);
                thread::sleep(time::Duration::from_secs(config.delay_request_data));
            }
            Ok(msg) => {
                error!("NCServerMessage missmatch, expected: HasData, got: {:?}", msg);
            }
            Err(e) => {
                error!("Error while getting new data from server: {}", e);
            }
        }
    }
}
