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
    debug!("Start nc_start_node()");

    let ip_addr: IpAddr = config.address.parse()?;
    let socket_addr = SocketAddr::new(ip_addr, config.port);

    let (node_id, initial_data) = get_initial_data(&socket_addr)?;

    nc_node.set_initial_data(node_id, initial_data)?;

    let heartbeat_duration = config.heartbeat;
    let heartbeat_handle = start_hearbeat_thread(node_id, socket_addr, heartbeat_duration);

    start_main_loop(nc_node, socket_addr, config, node_id);

    heartbeat_handle.join().map_err(|_| NCError::ThreadJoin)?;
    Ok(())
}

fn get_initial_data(socket_addr: &SocketAddr) -> Result<(NodeID, Vec<u8>), NCError> {
    debug!("Start get_initial_data(), socket_addr: {}", socket_addr);

    let initial_data = nc_send_receive_data(&NCNodeMessage::Register, socket_addr)?;

    match initial_data {
        NCServerMessage::InitialData(node_id, initial_data) => {
            Ok((node_id, initial_data))
        }
        msg => {
            error!("Error in get_initial_data(), NCServerMessage missmatch, expected: InitialData, got: {:?}", msg);
            Err(NCError::ServerMsgMismatch)
        }
    }
}

fn start_hearbeat_thread(node_id: NodeID, socket_addr: SocketAddr, heartbeat_duration: u64) -> thread::JoinHandle<()> {
    debug!("Start start_hearbeat_thread(), node_id: {:?}, heartbeat_duration: {}", node_id, heartbeat_duration);

    let duration = time::Duration::from_secs(heartbeat_duration);

    thread::spawn(move || {
        loop {
            thread::sleep(duration);

            match send_heartbeat(node_id, socket_addr) {
                Ok(quit) => {
                    if quit { break }
                }
                Err(e) => {
                    error!("Error in send_hearbeat: {}", e);
                }
            }
        }
    })
}

fn send_heartbeat(node_id: NodeID, socket_addr: SocketAddr) -> Result<bool, NCError> {
    debug!("Start send_heartbeat(), node_id: {:?}, socket_addr: {}", node_id, socket_addr);

    let result = nc_send_receive_data(&NCNodeMessage::HeartBeat(node_id), &socket_addr)?;

    match result {
        NCServerMessage::HeartBeatOK(NCJobStatus::Finished) => {
            debug!("Job is done, heartbeats no longer needed");
            Ok(true)
        }
        NCServerMessage::HeartBeatOK(_) => {
            debug!("Got heartbeat OK from server");
            Ok(false)
        }
        msg => {
            debug!("NCServerMessage missmatch, expected: HeartBeatOK, got: {:?}", msg);
            Err(NCError::ServerMsgMismatch)
        }
    }
}

fn start_main_loop<T: NCNode>(mut nc_node: T, socket_addr: SocketAddr, config: NCConfiguration, node_id: NodeID) {
    debug!("Start start_main_loop(), socket_addr: {}, node_id: {:?}", socket_addr, node_id);

    let duration = time::Duration::from_secs(config.delay_request_data);

    loop {
        debug!("Ask server for new data");

        match get_and_process_data(&mut nc_node, socket_addr, node_id) {
            Ok(quit) => {
                if quit { break }
            }
            Err(e) => {
                error!("Error in get_and_process_data(): {}", e);
                debug!("Will try again in {} seconds (delay_request_data)", config.delay_request_data);
                thread::sleep(duration);
            }
        }
    }
}

fn get_and_process_data<T: NCNode>(nc_node: &mut T, socket_addr: SocketAddr, node_id: NodeID) -> Result<bool, NCError> {
    debug!("Start get_and_process_data(), socket_addr: {}, node_id: {:?}", socket_addr, node_id);

    let result = nc_send_receive_data(&NCNodeMessage::NeedsData(node_id), &socket_addr)?;

    match result {
        NCServerMessage::HasData(data) => {
            debug!("New data received from server");
            let result = nc_node.process_data_from_server(data)?;
            nc_send_data(&result, &socket_addr).map(|_| false)
        }
        NCServerMessage::Waiting => {
            // The node will not exit here since the job is not 100% done.
            // This just means that all the remaining work has already
            // been distributed among all nodes.
            // One of the nodes can still crash and thus free nodes have to ask the server for more work
            // from time to time (delay_request_data).

            debug!("Waiting for other nodes to finish...");
            Ok(false)
        }
        NCServerMessage::Finished => {
            debug!("Job is done, exit now.");
            Ok(true)
        }
        NCServerMessage::PrepareDataError => {
            debug!("Error while preparing data for node.");
            Err(NCError::PrepareData)
        }
        msg => {
            debug!("NCServerMessage missmatch, expected: HasData, got: {:?}", msg);
            Err(NCError::ServerMsgMismatch)
        }
    }
}
