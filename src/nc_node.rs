use std::net::{IpAddr, SocketAddr};
use std::{thread, time::Duration};

use log::{error, debug};

use serde::{Serialize, Deserialize};

use crate::nc_error::{NCError};
use crate::nc_server::{NCServerMessage, NCJobStatus};
use crate::nc_config::{NCConfiguration};
use crate::nc_node_info::{NodeID};
use crate::nc_util::{nc_send_receive_data, nc_send_data};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCNodeMessage {
    Register,
    NeedsData(NodeID),
    HasData(NodeID, Vec<u8>),
    HeartBeat(NodeID),
}

// TODO: Generic trait, U for data in, V for data out
pub trait NCNode {
    fn set_initial_data(&mut self, node_id: NodeID, initial_data: Option<Vec<u8>>) -> Result<(), NCError> {
        Ok(())
    }
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Result<Vec<u8>, NCError>;
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

    debug!("Exit nc_start_node()");
    Ok(())
}

fn get_initial_data(socket_addr: &SocketAddr) -> Result<(NodeID, Option<Vec<u8>>), NCError> {
    debug!("Start get_initial_data(), socket_addr: {}", socket_addr);

    let initial_data = nc_send_receive_data(&NCNodeMessage::Register, socket_addr)?;

    match initial_data {
        NCServerMessage::InitialData(node_id, initial_data) => {
            debug!("Got node_id: {} and initial data from server", node_id);
            Ok((node_id, initial_data))
        }
        msg => {
            error!("Error in get_initial_data(), NCServerMessage missmatch, expected: InitialData, got: {:?}", msg);
            Err(NCError::ServerMsgMismatch)
        }
    }
}

fn start_hearbeat_thread(node_id: NodeID, socket_addr: SocketAddr, heartbeat_duration: u64) -> thread::JoinHandle<()> {
    debug!("Start start_hearbeat_thread(), node_id: {}, heartbeat_duration: {}", node_id, heartbeat_duration);

    let duration = Duration::from_secs(heartbeat_duration);
    let mut error_counter = 0;

    thread::spawn(move || {
        loop {
            thread::sleep(duration);

            match nc_send_receive_data(&NCNodeMessage::HeartBeat(node_id), &socket_addr) {
                Ok(NCServerMessage::HeartBeat(quit)) => {
                    error_counter = 0;
                    if quit { break }
                }
                Ok(msg) => {
                    error!("Error in start_heartbeat_thread(), unexpected server message: {:?}", msg);
                }
                Err(e) => {
                    error!("Error in start_heartbeat_thread(): {}", e);

                    error_counter += 1;
                    if error_counter >= 10 { // TODO: Make this configurable
                        error!("Error in start_heartbeat_thread(), error_counter limit exceeded: {}", error_counter);
                        break;
                    }
                }
            }
        }
    })
}

fn start_main_loop<T: NCNode>(mut nc_node: T, socket_addr: SocketAddr, config: NCConfiguration, node_id: NodeID) {
    debug!("Start start_main_loop(), socket_addr: {}, node_id: {}", socket_addr, node_id);

    let duration = Duration::from_secs(config.delay_request_data);

    loop {
        debug!("Ask server for new data");

        match get_and_process_data(&mut nc_node, socket_addr, node_id, duration) {
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

    debug!("Main loop finished")
}

fn get_and_process_data<T: NCNode>(nc_node: &mut T, socket_addr: SocketAddr, node_id: NodeID, duration: Duration) -> Result<bool, NCError> {
    debug!("Start get_and_process_data(), socket_addr: {}, node_id: {}", socket_addr, node_id);

    let result = nc_send_receive_data(&NCNodeMessage::NeedsData(node_id), &socket_addr)?;

    if let NCServerMessage::JobStatus(job_status) = result {
        match job_status {
            NCJobStatus::Unfinished(data) => {
                debug!("New data received from server");
                let result = nc_node.process_data_from_server(data)?;
                debug!("Send processed data to server");
                nc_send_data(&NCNodeMessage::HasData(node_id, result), &socket_addr).map(|_| false)
            }
            NCJobStatus::Waiting => {
                // The node will not exit here since the job is not 100% done.
                // This just means that all the remaining work has already
                // been distributed among all nodes.
                // One of the nodes can still crash and thus free nodes have to ask the server for more work
                // from time to time (delay_request_data).

                debug!("Waiting for other nodes to finish (delay_request_data: {})...", duration.as_secs());
                thread::sleep(duration);
                Ok(false)
            }
            NCJobStatus::Finished => {
                debug!("Job is done, exit now.");
                Ok(true)
            }
        }
    } else {
        error!("Error in get_and_process_data(), NCServerMessage missmatch, expected: JobStatus, got: {:?}", result);
        Err(NCError::ServerMsgMismatch)
    }
}
