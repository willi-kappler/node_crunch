//! This module contains the nc node message, trait and helper functions
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
    /// This is the first thing every node has to do!
    Register,
    /// This node needs new data to process. The server answers with a JobStatus message.
    NeedsData(NodeID),
    /// This node has finished processing the data and sends it to the server. No answer from the server.
    HasData(NodeID, Vec<u8>),
    /// This node sends a heartbeat message every n seconds. The time span between two heartbeats is set in the configuration NCConfiguration.
    HeartBeat(NodeID),
    /// This is a message that the server sends to itself to break out from blocking on node connection via accept()
    WakeUpServer,
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
    fn process_data_from_server(&mut self, data: &[u8]) -> Result<Vec<u8>, NCError>;
}

/// The main entry point for the code that runs on all nodes.
/// You give it your own user defined data structure that implements the NCNode trait and the configuration
/// Everything else is done automatically for you.
/// The NCNode trait function set_initial_data() is called here once in order to set the node id and some optional data that is
/// the same for all nodes at the beginning.
pub fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    debug!("Start nc_start_node()");

    let ip_addr: IpAddr = config.address.parse()?;
    let socket_addr = SocketAddr::new(ip_addr, config.port);
    let (node_id, initial_data) = get_initial_data(&socket_addr)?;

    nc_node.set_initial_data(node_id, initial_data)?;

    let retry_counter = RetryCounter::new(config.retry_counter);

    crossbeam::scope(|scope|{
        start_heartbeat_thread(scope, node_id, socket_addr, Duration::from_secs(config.heartbeat), retry_counter.clone());
        start_main_loop(nc_node, socket_addr, config, node_id, retry_counter);
    }).unwrap();

    info!("Job done, exit now");
    Ok(())
}

/// This is called once at the beginning of nc_start_node().
/// It sends a NCNodeMessage::Register message to the server and expects a NCServerMessage::InitialData message from the server.
/// On succeeded it returns the new assigned node id for this node and an optional initial data.
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

/// The heartbeat thread that runs in the background and sends heartbeat messages to the server is started here.
/// It does this every n seconds which can be configured in the NCConfiguration data structure.
/// If the server doesn't receive the heartbeat within the valid time span, the server marks the node internally as offline
/// and gives another node the same data chunk to process.
fn start_heartbeat_thread(scope: &Scope, node_id: NodeID, socket_addr: SocketAddr, heartbeat_duration: Duration, mut retry_counter: RetryCounter) {
    debug!("Start start_heartbeat_thread(), node_id: {}, heartbeat_duration: {}", node_id, heartbeat_duration.as_secs());

    scope.spawn(move |_| {
        loop {
            thread::sleep(heartbeat_duration);

            if let Err(e) = nc_send_data(&NCNodeMessage::HeartBeat(node_id), &socket_addr) {
                error!("Error in nc_send_data(): {}, retry_counter: {:?}", e, retry_counter);

                if retry_counter.dec_and_check() {
                    debug!("Retry counter is zero, will exit now");
                    break
                }
            } else {
                // Reset the counter if message was sent sucessfully
                retry_counter.reset()
            }
        }
    });
}

/// Here is main loop for this node. It keeps requesting and processing data until the server
/// sends a NCJobStatus::Finished message to this node.
/// If there is an error this node will wait n seconds before it tries to reconnect to the server.
/// The delay time can be configured in the NCConfiguration data structure.
/// With every error the retry counter is decremented. If it reaches zero the node will give up and exit.
/// The counter can be configured in the NCConfiguration.
fn start_main_loop<T: NCNode>(mut nc_node: T, socket_addr: SocketAddr, config: NCConfiguration, node_id: NodeID, mut retry_counter: RetryCounter) {
    debug!("Start start_main_loop(), socket_addr: {}, node_id: {}", socket_addr, node_id);

    let duration = Duration::from_secs(config.delay_request_data);

    loop {
        debug!("Ask server for new data");

        if let Err(e) = get_and_process_data(&mut nc_node, socket_addr, node_id, duration) {
            error!("Error in get_and_process_data(): {}, retry counter: {:?}", e, retry_counter);

            if retry_counter.dec_and_check() {
                debug!("Retry counter is zero, will exit now");
                break
            }

            debug!("Will wait before retry (delay_request_data: {} sec)", duration.as_secs());
            thread::sleep(duration);
        } else {
            // Reset the counter if message was sent sucessfully
            retry_counter.reset()
        }
    }

    debug!("Main loop finished")
}

/// This function sends a NCNodeMessage::NeedsData message to the server and reacts accordingly to the server response:
/// Only one message is expected as a response from the server: NCServerMessage::JobStatus. This status can have two values
/// 1. NCJobStatus::Unfinished: This means that the job is note done and there is still some more data to be processed.
///      This node will then process the data calling the process_data_from_server() function and sends the data back to the
///      server using the NCNodeMessage::HasData message.
/// 2. NCJobStatus::Waiting: This means that not all nodes are done and the server is still waiting for all nodes to finish.
/// If the server sends a different message this function will return a NCError::ServerMsgMismatch error.
fn get_and_process_data<T: NCNode>(nc_node: &mut T, socket_addr: SocketAddr, node_id: NodeID, duration: Duration) -> Result<(), NCError> {
    debug!("Start get_and_process_data(), socket_addr: {}, node_id: {}", socket_addr, node_id);

    let result = nc_send_receive_data(&NCNodeMessage::NeedsData(node_id), &socket_addr)?;

    if let NCServerMessage::JobStatus(job_status) = result {
        match job_status {
            NCJobStatus::Unfinished(data) => {
                debug!("New data received from server");
                let result = nc_node.process_data_from_server(&data)?;
                debug!("Send processed data to server");
                nc_send_data(&NCNodeMessage::HasData(node_id, result), &socket_addr)
            }
            NCJobStatus::Waiting => {
                // The node will not exit here since the job is not 100% done.
                // This just means that all the remaining work has already
                // been distributed among all nodes.
                // One of the nodes can still crash and thus free nodes have to ask the server for more work
                // from time to time (delay_request_data).

                debug!("Waiting for other nodes to finish (delay_request_data: {} sec)...", duration.as_secs());
                thread::sleep(duration);
                Ok(())
            }
            msg => {
                // The server does not bother sending the node a NCJobStatus::Finished message.
                error!("Error: unexpected message from server: {:?}", msg);
                Err(NCError::ServerMsgMismatch)
            }
        }
    } else {
        error!("Error in get_and_process_data(), NCServerMessage mismatch, expected: JobStatus, got: {:?}", result);
        Err(NCError::ServerMsgMismatch)
    }
}

/// Counter for nc_node if connection to server is not possible.
/// The counter will be decreased every time there is an IO error and if it is zero the function dec_and_check
/// returns true, otherwise false.
/// When the connection to the server is working again, the counter is reset to its initial value.
#[derive(Debug, Clone)]
pub(crate) struct RetryCounter {
    init: u64,
    counter: u64,
}

impl RetryCounter {
    /// Create a new retry counter with the given limit.
    /// It will count backwards to zero.
    pub(crate) fn new(counter: u64) -> Self {
        RetryCounter{ init: counter, counter }
    }

    /// Decrements and checks the counter.
    /// If it's zero return true, else return false.
    pub(crate) fn dec_and_check(&mut self) -> bool {
        if self.counter == 0 {
            true
        } else {
            self.counter -= 1;
            false
        }
    }

    /// Resets the counter to it's initla value.
    pub(crate) fn reset(&mut self) {
        self.counter = self.init
    }
}
