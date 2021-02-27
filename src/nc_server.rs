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
    /// For exmaple a 2D array can be split up into smaller pieces that are processed by each node.
    /// Usually the server will have an internal data structure containing all the registered nodes.
    /// According to the status of the job this function returns a NCJobStatus value:
    /// Unfinished, Waiting or Finished.
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus, NCError>;
    /// When one node is done processing the data from the server it will send the result back to the server and then this function is called.
    /// For example a small piece of a 2D array may be returned by the node and the server puts the resulting data back into the big 2D array.
    fn process_data_from_node(&mut self, node_id: NodeID, data: &[u8]) -> Result<(), NCError>;
    /// Every node has to send a heartbeat message to the server. If it doesn't arrive in time (2 * the heartbeat value in the NCConfiguration)
    /// then this function is called with the corresponding node id and the node should be marked as offline in this function.
    fn heartbeat_timeout(&mut self, nodes: Vec<NodeID>);
    /// When all the nodes are done with processing and all internal threads are also finished then this function is called.
    /// Usually you want to save all the results to disk and optionally you can write an e-mail to the user that he / she can start
    /// writing a paper for his / her PhD.
    fn finish_job(&mut self);
}

/// Main data structure for managing and starting the server.
pub struct NCServerStarter {
    /// Configuration for the server and the node.
    config: NCConfiguration,
}

impl NCServerStarter {
    /// Create a new NCServerStarter using the given configuration
    pub fn new(config: NCConfiguration) -> Self {
        debug!("NCServerStarter::new()");

        NCServerStarter{ config }
    }

    /// This is the main function that you call when you start the server. It expects your custom data structure that implements the NCServer trait.
    pub fn start<T: NCServer + Send>(&mut self, nc_server: T) -> Result<(), NCError> {
        debug!("NCServerStarter::new()");

        let time_start = Instant::now();
        // let job_done = Arc::new(AtomicBool::new(false));

        let server_process = ServerProcess::new(self.config.clone(), nc_server);
        let server_heartbeat = ServerHeartbeat::new(self.config.clone());

        crossbeam::scope(|scope|{
            self.start_heartbeat_thread(scope, server_heartbeat);
            self.start_main_loop(scope, server_process);
        }).unwrap();

        let time_taken = (Instant::now() - time_start).as_secs_f64();

        info!("Time taken: {} s, {} min, {} h", time_taken, time_taken / 60.0, time_taken / (60.0 * 60.0));

        Ok(())
    }

    /// The heartbeat check thread is started here in an endless loop.
    /// It calls the function send_check_heartbeat_message() which sends the NCNodeMessage::CheckHeartbeat message
    /// to the server. The server then checks all the nodes to see if one of them missed a heartbeat.
    /// If there is an IO error the loop exits because the server also has finished its main loop and
    /// doesn't accept any tcp connections anymore.
    /// The job is done and no more heartbeats will arrive.
    fn start_heartbeat_thread(&self, scope: &Scope, server_heartbeat: ServerHeartbeat) {
        debug!("NCServerStarter::start_heartbeat_thread()");

        scope.spawn(move |_| {
            loop {
                server_heartbeat.sleep();

                if let Err(e) = server_heartbeat.send_check_heartbeat_message() {
                    error!("Error in start_heartbeat_thread(), couldn't send CheckHeartbeat message: {}", e);
                    break
                }
            }
            debug!("Exit start_heartbeat_thread() main loop");
        });
    }

    /// In here the main loop and the tcp server are started.
    /// For every node connection the function start_node_thread() is called, which handles the node request in a separate thread.
    /// If the job is done one the main loop will exited
    fn start_main_loop<'a, T: 'a + NCServer + Send>(&self, scope: &Scope<'a>, server_process: ServerProcess<T>) {
        debug!("NCServerStarter::start_main_loop()");

        let ip_addr: IpAddr = "0.0.0.0".parse().unwrap(); // TODO: Make this configurable
        let socket_addr = SocketAddr::new(ip_addr, server_process.config.port);
        let listener = TcpListener::bind(socket_addr).unwrap();

        let job_done = server_process.clone_job_done();

        let server_process = Arc::new(server_process);

        loop {
            match listener.accept() {
                Ok((stream, addr)) => {
                    debug!("Connection from node: {}", addr);
                    self.start_node_thread(scope, stream, server_process.clone());
                }
                Err(e) => {
                    error!("IO error while accepting node connections: {}", e);
                }
            }

            if job_done.load(Ordering::Relaxed) {
                // Try to exit main loop as soon as possible.
                // Don't bother with informing the nodes since they have a retry counter for IO errors
                // and will exit when the counter reaches zero.
                // The server check_heartbeat thread also exits if there is an IO error, that means the server
                // doesn't accept connections anymore.
                break
            }
        }

        info!("Job is done, will call NCServer::finish_job()");
        server_process.nc_server.lock().unwrap().finish_job();
    }
    /// This starts a new thread for each node that sends a message to the server and calls the handle_node() function in that thread.
    fn start_node_thread<'a, T: 'a + NCServer + Send>(&self, scope: &Scope<'a>, stream: TcpStream, server_process: Arc<ServerProcess<T>>) {
        debug!("NCServerStarter::start_node_thread()");

        scope.spawn(move |_| {
            if let Err(e) = server_process.handle_node(stream) {
                error!("Error in handle_node(): {}", e);
            }
        });
    }
}

/// TODO:
struct ServerHeartbeat {
    /// TODO:
    server_socket: SocketAddr,
    /// TODO:
    duration: Duration,
}

impl ServerHeartbeat {
    /// TODO:
    fn new(config: NCConfiguration) -> Self {
        debug!("ServerHeartbeat::new()");

        let ip_addr: IpAddr = "127.0.0.1".parse().unwrap();
        let server_socket = SocketAddr::new(ip_addr, config.port);
        let duration = Duration::from_secs(2 * config.heartbeat);

        ServerHeartbeat{
            server_socket,
            duration,
        }
    }

    /// TODO:
    fn sleep(&self) {
        debug!("ServerHeartbeat::sleep()");

        thread::sleep(self.duration);
    }

    /// TODO:
    fn send_check_heartbeat_message(&self) -> Result<(), NCError> {
        debug!("ServerHeartbeat::send_check_heartbeat_message()");

        nc_send_data(&NCNodeMessage::CheckHeartbeat, &self.server_socket)
    }
}

/// TODO:
struct ServerProcess<T> {
    /// The server and node configuration.
    config: NCConfiguration,
    /// The user defined data structure that implements the NCServer trait.
    nc_server: Mutex<T>,
    /// Internal list of all the registered nodes.
    node_list: Mutex<NCNodeList>,
    /// Indicates if the job is already done and the server can exit its main loop.
    job_done: Arc<AtomicBool>,
}

impl<T: NCServer> ServerProcess<T> {
    /// Creates a new ServerProcess with the given user defined nc_server that implements the NCServer trait
    fn new(config: NCConfiguration, nc_server: T) -> Self {
        debug!("ServerProcess::new()");

        ServerProcess{
            config,
            nc_server: Mutex::new(nc_server),
            node_list: Mutex::new(NCNodeList::new()),
            job_done: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Clones the job_done AtomicBool and returns it
    fn clone_job_done(&self) -> Arc<AtomicBool> {
        debug!("ServerProcess::clone_job_done()");

        self.job_done.clone()
    }

    /// All the message that were sent from a node are handled here. It can be on of these types:
    /// - NCNodeMessage::Register: every new node has to register first, the server then assigns a new node id and sends some optional initial data back to the node with the
    ///   NCServerMessage::InitialData message. The server trait function initial_data() is called here.
    /// - NCNodeMessage::NeedsData: the node needs some data to process and depending on the job state the server answers this request with a NCServerMessage::JobStatus message.
    ///   The server trait function prepare_data_for_node() is called here.
    /// - NCNodeMessage::HeartBeat: the node sends a heartbeat message and the server updates the internal node list with the corresponding current time stamp.
    /// - NCNodeMessage::HasData: the node has finished processing the data and has sent the result back to the server.
    ///   The server trait function process_data_from_node() is called here.
    /// - NCNodeMessage::CheckHeartbeat: This message is sent from the check heartbeat thread to the server
    ///   itself. All the nodes will be checked for the heartbeat time stamp and if a node missed it, the NCServer trait
    ///   function heartbeat_timeout() is called where the node should be marked as offline.
    fn handle_node(&self, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::handle_node()");

        let request: NCNodeMessage = nc_receive_data(&mut stream)?;

        match request {
            NCNodeMessage::Register => {
                let node_id = self.node_list.lock()?.register_new_node();
                let initial_data = self.nc_server.lock()?.initial_data()?;
                info!("Registering new node: {}, {}", node_id, stream.peer_addr()?);
                self.send_initial_data_message(node_id, initial_data, stream)?;
                // nc_send_data2(&NCServerMessage::InitialData(node_id, initial_data), &mut stream)?;
            }
            NCNodeMessage::NeedsData(node_id) => {
                debug!("Node {} needs data to process", node_id);
                let data_for_node = self.nc_server.lock()?.prepare_data_for_node(node_id)?;

                match data_for_node {
                    NCJobStatus::Unfinished(data) => {
                        debug!("Send data to node");
                        self.send_job_status_unfinished_message(data, stream)?;
                        // nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Unfinished(data)), &mut stream)?;
                        debug!("Data has been sent to node");
                    }
                    NCJobStatus::Waiting => {
                        debug!("Waiting for other nodes to finish");
                        self.send_job_status_waiting(stream)?;
                        // nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Waiting), &mut stream)?;
                    }
                    NCJobStatus::Finished => {
                        debug!("Job is done, will exit handle_node()");
                        // Do not bother sending a message to the nodes, they will quit anyways after the retry counter is zero.
                        // The counter will be decremented if there is an IO error.
                        // Same for the server heartbeat thread, it will exit its loop if there is an IO error.
                        self.job_done.store(true, Ordering::Relaxed);
                    }
                }
            }
            NCNodeMessage::HeartBeat(node_id) => {
                debug!("Got heartbeat from node: {}", node_id);
                self.node_list.lock()?.update_heartbeat(node_id);
            }
            NCNodeMessage::HasData(node_id, data) => {
                debug!("Node {} has processed some data and we received the results", node_id);
                self.nc_server.lock()?.process_data_from_node(node_id, &data)?;
            }
            NCNodeMessage::CheckHeartbeat => {
                debug!("Messag CheckHeartbeat received!");
                // Check the heartbeat for all the nodes and call the trait function heartbeat_timeout()
                // with those nodes to react accordingly.
                let nodes = self.node_list.lock()?.check_heartbeat(self.config.heartbeat).collect::<Vec<NodeID>>();
                self.nc_server.lock()?.heartbeat_timeout(nodes);
            }
        }
        Ok(())
    }

    /// Sends the NCServerMessage::InitialData message to the node with the given node_id and optional initial_data
    fn send_initial_data_message(&self, node_id: NodeID, initial_data: Option<Vec<u8>>, mut stream: TcpStream) -> Result<(), NCError> {
        nc_send_data2(&NCServerMessage::InitialData(node_id, initial_data), &mut stream)
    }

    /// Sends the NCServerMessage::JobStatus Unfinished message with the given data to the node,
    fn send_job_status_unfinished_message(&self, data: Vec<u8>, mut stream: TcpStream) -> Result<(), NCError> {
        nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Unfinished(data)), &mut stream)
    }

    /// Send the NCServerMessage::JobStatus Waiting message to the node.
    fn send_job_status_waiting(&self, mut stream: TcpStream) -> Result<(), NCError> {
        nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Waiting), &mut stream)
    }
}
