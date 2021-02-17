//! This module contains the nc node data structure and helper functions
//! To use the node you have to implement the NCNode trait that has two functions:
//! set_initial_data() and process_data_from_server()

use std::net::{IpAddr, SocketAddr};
use std::{thread, time::Duration};

use log::{error, info, debug};

use serde::{Serialize, Deserialize};
use crossbeam::{self, thread::Scope};

use crate::nc_error::NCError;
use crate::nc_server::{NCServerMessage, NCJobStatus};
use crate::nc_config::NCConfiguration;
use crate::nc_node_info::NodeID;
use crate::nc_util::{nc_send_receive_data, nc_send_data};

/// This message is sent from the node to the server in order to register, receive new data and send processed data.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCNodeMessage {
    /// Register this node with the server. The server will assign a new node id to this node and answers with a NCServerMessage::InitialData message.
    Register,
    /// This node needs new data to process. The server answers with a JobStatus message.
    NeedsData(NodeID),
    /// This node has finished processing the data and sends it to the server. No answer from the server.
    HasData(NodeID, Vec<u8>),
    /// This node sends a hearbeat message every n seconds. The time span between two heartbeats is set in the configuration NCConfiguration.
    HeartBeat(NodeID),
    /// If the server sends a NCJobStatus::Finished message, this node responds with a NodeQuit message. This gives the server a chance to wait for all threads to finish.
    NodeQuit(NodeID),
}

// TODO: Generic trait, U for data in, V for data out
/// This trait has to be implemented for the code that runs on all the nodes.
pub trait NCNode {
    /// Once this node has sent a NCNodeMessage::Register message the server responds with a NCServerMessage::InitialData message.
    /// Then this function is called with the data received from the server.
    fn set_initial_data(&mut self, node_id: NodeID, initial_data: Option<Vec<u8>>) -> Result<(), NCError> {
        debug!("Got new node id: {}", node_id);
        match initial_data {
            Some(_) => debug!("Got some initial data from the server."),
            None => debug!("Got no initial data from the server.")
        }

        Ok(())
    }

    /// Whenever the node requests new data from the server, the server will respond with new data that needs to be processed by the node.
    /// This function is then called with the data that was received from the server.
    /// Here you put your code that does the main number crunching on every node.
    /// Note that you have to use the nc_decode_data() or nc_decode_data2() helper functions from the nc_utils module in order to
    /// deserialize the data.
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Result<Vec<u8>, NCError>;
}

/// This function is the main entry point for the code that runs on all nodes.
/// You give it your own user defined data structure that implements the NCNode trait and the configuration
/// Everything else is done automatically for you.
pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    debug!("Start nc_start_node()");

    let ip_addr: IpAddr = config.address.parse()?;
    let socket_addr = SocketAddr::new(ip_addr, config.port);

    let (node_id, initial_data) = get_initial_data(&socket_addr)?;

    nc_node.set_initial_data(node_id, initial_data)?;

    crossbeam::scope(|scope|{
        start_heartbeat_thread(scope, node_id, socket_addr, Duration::from_secs(config.heartbeat));
        start_main_loop(nc_node, socket_addr, config, node_id);
    }).unwrap();

    info!("Job done, exit now");
    Ok(())
}

/// This funcrion is called once at the beginning of nc_start_node().
/// It sends a NCNodeMessage::Register message to the server and expects a NCServerMessage::InitialData message from the server.
/// On succedd it returns the new assigned node id for this node and an optional initial data.
/// If the server doesn't respond with a NCServerMessage::InitialData message a NCError::ServerMsgMismatch error is returned.
fn get_initial_data(socket_addr: &SocketAddr) -> Result<(NodeID, Option<Vec<u8>>), NCError> {
    debug!("Start get_initial_data(), socket_addr: {}", socket_addr);

    let initial_data = nc_send_receive_data(&NCNodeMessage::Register, socket_addr)?;

    match initial_data {
        NCServerMessage::InitialData(node_id, initial_data) => {
            info!("Got node_id: {} and initial data from server", node_id);
            Ok((node_id, initial_data))
        }
        msg => {
            error!("Error in get_initial_data(), NCServerMessage mismatch, expected: InitialData, got: {:?}", msg);
            Err(NCError::ServerMsgMismatch)
        }
    }
}

/// This function starts the heartbeat thread that runs in the background and sends heartbeat messages to the server.
/// It does this every n seconds which can be configured in the NCConfiguration data structure.
/// If the server doesn't receive the heartbeat within the valid time span, the server marks the node internally as offline
/// and gives another node the same data chunk to process.
fn start_heartbeat_thread(scope: &Scope, node_id: NodeID, socket_addr: SocketAddr, heartbeat_duration: Duration) {
    debug!("Start start_heartbeat_thread(), node_id: {}, heartbeat_duration: {}", node_id, heartbeat_duration.as_secs());

    scope.spawn(move |_| {
        loop {
            thread::sleep(heartbeat_duration);

            match nc_send_receive_data(&NCNodeMessage::HeartBeat(node_id), &socket_addr) {
                Ok(NCServerMessage::HeartBeat(quit)) => {
                    if quit { break }
                }
                Ok(msg) => {
                    error!("Error in start_heartbeat_thread(), unexpected server message: {:?}", msg);
                }
                Err(e) => {
                    error!("Error in nc_send_receive_data(): {}", e);
                }
            }
        }
    });
}

/// This function starts the main loop for this node. It keeps requesting and processing data until the server
/// sends a NCJobStatus::Finished message to this node.
/// If there is an error this node will wait n seconds before it trys to reconnect to the server.
/// The delay time can be configures in the NCConfiguration data structure.
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
                debug!("Will wait before retry (delay_request_data: {} sec)", duration.as_secs());
                thread::sleep(duration);
            }
        }
    }

    debug!("Main loop finished")
}

/// This function sends a NCNodeMessage::NeedsData message to the server and reacts accordingly to the server response:
/// Only one message is expected as a response from the server: NCServerMessage::JobStatus. This status can have three values
/// 1. NCJobStatus::Unfinished: This means that the job is note done and there is still some more data to be processed.
///      This node will then process the data calling the process_data_from_server() function and sends the data back to the
///      server using the NCNodeMessage::HasData message.
/// 2. NCJobStatus::Waiting: This means that not all nodes are done and the server is still waiting for all nodes to finish.
/// 3. NCJobStatus::Finished: The job is finally done and all the nodes can exit.
/// If the server sends a different message this function will return a NCError::ServerMsgMismatch error.
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

                debug!("Waiting for other nodes to finish (delay_request_data: {} sec)...", duration.as_secs());
                thread::sleep(duration);
                Ok(false)
            }
            NCJobStatus::Finished => {
                debug!("Job is done, node will quit.");
                // This gives the server a chance to exit the main loop
                nc_send_data(&NCNodeMessage::NodeQuit(node_id), &socket_addr).map(|_| false)?;
                Ok(true)
            }
        }
    } else {
        error!("Error in get_and_process_data(), NCServerMessage mismatch, expected: JobStatus, got: {:?}", result);
        Err(NCError::ServerMsgMismatch)
    }
}
