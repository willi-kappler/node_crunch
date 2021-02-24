//! This module contains the nc server message, trait and helper functions
//! To use the server you have to implement the NCServer trait that has five functions:
//! initial_data(): This function is called once for every node when the node registers with the server.
//! prepare_data_for_node(): This function is called when the node needs new data to process.
//! process_data_from_node(): This function is called when the node is done with processing the data and has sent the result back to the server.
//! heartbeat_timeout(): This function is called when the node has missed a heartbeat, usually the node is then marked as offline and the chunk
//!     of data for that node is sent to another node.
//! finish_job(): This function is called when the job is done and all the threads are finished. Usually you want to save the results to disk
//!     in here.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::time::{Instant, Duration};

use log::{error, info, debug};
use serde::{Serialize, Deserialize};
use crossbeam::{self, thread::Scope};

use crate::nc_error::NCError;
use crate::nc_node::NCNodeMessage;
use crate::nc_config::NCConfiguration;
use crate::nc_node_info::{NodeID, NCNodeList};
use crate::nc_util::{nc_receive_data, nc_send_data, nc_send_data2};

/// This message is send from the server to each node. It can be some initial data, the job status or a heartbeat response.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCServerMessage {
    /// When the node registers for the first time with the NCNodeMessage::Register message the server assigns a new node id
    /// and sends some optional initial data to the node.
    InitialData(NodeID, Option<Vec<u8>>),
    /// When the node requests new data to process wth the NCNodeMessage::NeedsData message, the current job status is sent to
    /// the node: unfinished, waiting or finished.
    JobStatus(NCJobStatus),
}

/// The job status tells the node what to do next: process the new data, wait for other nodes to finish or exit. This is the answer from the server when
/// a node request new data via the NCNodeMessage::NeedsData message.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum NCJobStatus {
    /// The job is not done yet and the node has to process the data the server sends to it.
    Unfinished(Vec<u8>),
    /// The server is still waiting for other nodes to finish the job. This means that all the work has already been distributed to all the nodes
    /// and the server sends this message to the remaining nodes. It does this because some of the processing nodes can still crash, so that its work
    /// has to be done by a waiting node.
    Waiting,
    /// Now all nodes are finished and the job is done. The server sends this message to all the nodes that request new data.
    Finished,
}

type NCNodeInfoList = Arc<Mutex<NCNodeList>>;

// TODO: Generic trait, U for data in, V for data out
/// This is the trait that you have to implement in order to start the server.
pub trait NCServer {
    /// This function is called once for every new node that registers with the server using the NCNodeMessage::Register message.
    /// It may prepare some initial data that is common for all nodes at the beginning of the job.
    fn initial_data(&mut self) -> Result<Option<Vec<u8>>, NCError> {
        Ok(None)
    }
    /// This function is called when the node requests new data with the NCNodeMessage::NeedsData message.
    /// It's the servers task to prepare the data for each node individually.
    /// Usually the server will have an internal data structure containing all the registered nodes.
    /// According to the status of the job this function returns a NCJobStatus value:
    /// Unfinished, Waiting or Finished.
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus, NCError>;
    /// When one node is done processing the data from the server it will send the result back to the server and then this function is called.
    fn process_data_from_node(&mut self, node_id: NodeID, data: &[u8]) -> Result<(), NCError>;
    /// Every node has to send a heartbeat message to the server. If it doesn't arrive in time (2 * the heartbeat value in the NCConfiguration)
    /// then this function is called with the corresponding node id and the node should marked as offline in this function.
    fn heartbeat_timeout(&mut self, nodes: Vec<NodeID>);
    /// When all the nodes are done with processing and all internal threads are also finished then this function is called.
    /// Usually you want to save all the results to disk and optionally you can write an e-mail to the user to get his a** off the couch
    /// and start writing a paper for his / her PhD.
    fn finish_job(&mut self);
}

/// This is the main function that you call when you start the server. It expects your custom data structure that implements the NCServer trait
/// and a configuration.
/// Once the job is done the server trait function finish_job() is called here.
pub fn nc_start_server<T: NCServer + Send>(nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    debug!("Start nc_start_server()");

    let time_start = Instant::now();
    let nc_server = Arc::new(Mutex::new(nc_server));
    let node_list = Arc::new(Mutex::new(NCNodeList::new()));
    let job_done = Arc::new(AtomicBool::new(false));

    crossbeam::scope(|scope|{
        start_heartbeat_thread(scope, 2 * config.heartbeat, node_list.clone(),
            nc_server.clone(), config.port);
        start_main_loop(scope, node_list, nc_server.clone(), config, job_done);
    }).unwrap();

    info!("Call finish_job() for nc_server");

    nc_server.lock()?.finish_job();

    let time_taken = (Instant::now() - time_start).as_secs_f64();

    info!("Job done, exit now");
    info!("Time taken: {} s, {} min, {} h", time_taken, time_taken / 60.0, time_taken / (60.0 * 60.0));

    Ok(())
}

/// The heartbeat check thread is started here in an endless loop.
/// It calls the function check_heartbeat() which checks the heartbeat time stamp for all nodes.
/// Also sends the message WakeUpServer to the server itself, in order to exit from the blocking accept() call.
/// If the job is done the loop will exit.
fn start_heartbeat_thread<'a, T: 'a + NCServer + Send>(scope: &Scope<'a>, heartbeat_duration: u64,
    node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>, port: u16) {
    debug!("Start start_heartbeat_thread(), heartbeat_duration: {}", heartbeat_duration);

    let ip_addr: IpAddr = "127.0.0.1".parse().unwrap();
    let socket_addr = SocketAddr::new(ip_addr, port);

    scope.spawn(move |_| {
        let duration = Duration::from_secs(heartbeat_duration);

        loop {
            thread::sleep(duration);

            // Check the heartbeat for all the nodes and call the trait function heartbeat_timeout()
            // with those nodes to react accordingly.
            let nodes = node_list.lock().unwrap().check_heartbeat(heartbeat_duration).collect::<Vec<NodeID>>();
            nc_server.lock().unwrap().heartbeat_timeout(nodes);

            // Send WakeUpServer message to server so that it can check whether to exit the main loop or not
            if let Err(e) = nc_send_data(&NCNodeMessage::WakeUpServer, &socket_addr) {
                error!("Error in start_heartbeat_thread(), couldn't send WakeUpServer message: {}", e);
                // The server main loop has already been left, so exit this loop also.
                break
            }
        }
        debug!("Exit start_heartbeat_thread() main loop");
    });
}

/// In here the main loop and the tcp server are started.
/// For every node connection the function start_node_thread() is called, which handles the node request in a separate thread.
/// If the job is done one the main loop will exited
fn start_main_loop<'a, T: 'a + NCServer + Send>(scope: &Scope<'a>, node_list: NCNodeInfoList,
    nc_server: Arc<Mutex<T>>, config: NCConfiguration, job_done: Arc<AtomicBool>) {
    debug!("Start start_main_loop()");

    let ip_addr: IpAddr = "0.0.0.0".parse().unwrap(); // TODO: Make this configurable
    let socket_addr = SocketAddr::new(ip_addr, config.port);
    let listener = TcpListener::bind(socket_addr).unwrap();

    loop {
        match listener.accept() {
            Ok((stream, addr)) => {
                debug!("Connection from node: {}", addr);
                start_node_thread(scope, stream, node_list.clone(), nc_server.clone(), job_done.clone());
            }
            Err(e) => {
                error!("IO error while accepting node connections: {}", e);
            }
        }

        if job_done.load(Ordering::Relaxed) {
            // Try to exit main loop as soon as possible.
            // Don't bother with informing the nodes since they have a retry counter for IO errors
            // and will exit when the counter reaches zero.
            break
        }
    }
}

/// This starts a new thread for each node that sends a message to the server and calls the handle_node() function in that thread.
fn start_node_thread<'a, T: 'a + NCServer + Send>(scope: &Scope<'a>, stream: TcpStream,
    node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>, job_done: Arc<AtomicBool>) {
    debug!("Start start_node_thread()");

    scope.spawn(move |_| {
        if let Err(e) = handle_node(stream, node_list, nc_server, job_done) {
            error!("Error in handle_node(): {}", e);
        }
    });
}

/// All the message that were sent from a node are handled here. It can be on of these types:
/// - NCNodeMessage::Register: every new node has to register first, the server then assigns a new node id and sends some optional initial data back to the node with the
///   NCServerMessage::InitialData message. The server trait function initial_data() is called here.
/// - NCNodeMessage::NeedsData: the node needs some data to process and depending on the job state the server answers this request with a NCServerMessage::JobStatus message.
///   The server trait function prepare_data_for_node() is called here.
/// - NCNodeMessage::HeartBeat: the node sends a heartbeat message and the server updates the internal node list with the corresponding current time stamp.
/// - NCNodeMessage::HasData: the node has finished processing the data and has sent the result back to the server.
///   The server trait function process_data_from_node() is called here.
/// - NCNodeMessage::WakeUpServer: This gives the server a chance to break out from the blocking accept() function.
fn handle_node<T: NCServer>(mut stream: TcpStream, node_list: NCNodeInfoList,
    nc_server: Arc<Mutex<T>>, job_done: Arc<AtomicBool>) -> Result<(), NCError> {
    debug!("Start handle_node()");
    let request: NCNodeMessage = nc_receive_data(&mut stream)?;

    match request {
        NCNodeMessage::Register => {
            let node_id = node_list.lock()?.register_new_node();
            let initial_data = nc_server.lock()?.initial_data()?;
            info!("Registering new node: {}, {}", node_id, stream.peer_addr()?);
            nc_send_data2(&NCServerMessage::InitialData(node_id, initial_data), &mut stream)?;
        }
        NCNodeMessage::NeedsData(node_id) => {
            debug!("Node {} needs data to process", node_id);
            let data_for_node = nc_server.lock()?.prepare_data_for_node(node_id)?;

            match data_for_node {
                NCJobStatus::Unfinished(data) => {
                    debug!("Send data to node");
                    nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Unfinished(data)), &mut stream)?;
                    debug!("Data has been sent to node");
                }
                NCJobStatus::Waiting => {
                    debug!("Waiting for other nodes to finish");
                    nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Waiting), &mut stream)?;
                }
                NCJobStatus::Finished => {
                    debug!("Job is done, will exit handle_node()");
                    // Do not bother sending a message to the nodes, they will quit anyways after the retry counter is zero.
                    job_done.store(true, Ordering::Relaxed);
                }
            }
        }
        NCNodeMessage::HeartBeat(node_id) => {
            debug!("Got heartbeat from node: {}", node_id);
            node_list.lock()?.update_heartbeat(node_id);
        }
        NCNodeMessage::HasData(node_id, data) => {
            debug!("Node {} has processed some data and we received the results", node_id);
            nc_server.lock()?.process_data_from_node(node_id, &data)?;
        }
        NCNodeMessage::WakeUpServer => {
            debug!("Message WakeUpServer received!");
        }
    }

    Ok(())
}
