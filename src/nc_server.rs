//! This module contains the nc server message, trait and helper methods
//! To use the server you have to implement the NCServer trait that has five methods:
//! initial_data(): This method is called once for every node when the node registers with the server.
//! prepare_data_for_node(): This method is called when the node needs new data to process.
//! process_data_from_node(): This method is called when the node is done with processing the data and has sent the result back to the server.
//! heartbeat_timeout(): This method is called when the node has missed a heartbeat, usually the node is then marked as offline and the chunk
//!     of data for that node is sent to another node.
//! finish_job(): This method is called when the job is done and all the threads are finished. Usually you want to save the results to disk
//!     in here.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::time::{Instant, Duration};

use log::{error, info, debug};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use threadpool::ThreadPool;

use crate::nc_error::NCError;
use crate::nc_node::NCNodeMessage;
use crate::nc_config::NCConfiguration;
use crate::nc_node_info::{NodeID, NCNodeList};
use crate::nc_communicator::{NCCommunicator};

/// This message is send from the server to each node.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCServerMessage<InitialDataT, NewDataT, CustomMessageT> {
    /// When the node registers for the first time with the NCNodeMessage::Register message the server assigns a new node id
    /// and sends some optional initial data to the node.
    InitialData(NodeID, Option<InitialDataT>),
    /// When the node requests new data to process wth the NCNodeMessage::NeedsData message, the current job status is sent to
    /// the node: unfinished, waiting or finished.
    JobStatus(NCJobStatus<NewDataT>),
    /// Send some statistics about the server to the node.
    Statistics(NCServerStatistics),
    /// Move all nodes to a new server.
    NewServer(String, u16),
    /// Send a custom message to one or all nodes.
    CustomMessage(CustomMessageT),
}

/// The job status tells the node what to do next: process the new data, wait for other nodes to finish or exit. This is the answer from the server when
/// a node request new data via the NCNodeMessage::NeedsData message.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum NCJobStatus<NewDataT> {
    /// The job is not done yet and the node has to process the data the server sends to it.
    Unfinished(NewDataT),
    /// The server is still waiting for other nodes to finish the job. This means that all the work has already been distributed to all the nodes
    /// and the server sends this message to the remaining nodes. It does this because some of the processing nodes can still crash, so that its work
    /// has to be done by a waiting node.
    Waiting,
    /// Now all nodes are finished and the job is done. The server sends this message to all the nodes that request new data.
    Finished,
}

/// This is the trait that you have to implement in order to start the server.
pub trait NCServer {
    type InitialDataT: Serialize + DeserializeOwned;
    type NewDataT: Serialize + DeserializeOwned;
    type ProcessedDataT: Serialize + DeserializeOwned;
    type CustomMessageT: Serialize + DeserializeOwned + Send + Clone;

    /// This method is called once for every new node that registers with the server using the NCNodeMessage::Register message.
    /// It may prepare some initial data that is common for all nodes at the beginning of the job.
    fn initial_data(&mut self) -> Result<Option<Self::InitialDataT>, NCError> {
        Ok(None)
    }
    /// This method is called when the node requests new data with the NCNodeMessage::NeedsData message.
    /// It's the servers task to prepare the data for each node individually.
    /// For example a 2D array can be split up into smaller pieces that are processed by each node.
    /// Usually the server will have an internal data structure containing all the registered nodes.
    /// According to the status of the job this method returns a NCJobStatus value:
    /// Unfinished, Waiting or Finished.
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus<Self::NewDataT>, NCError>;
    /// When one node is done processing the data from the server it will send the result back to the server and then this method is called.
    /// For example a small piece of a 2D array may be returned by the node and the server puts the resulting data back into the big 2D array.
    fn process_data_from_node(&mut self, node_id: NodeID, data: &Self::ProcessedDataT) -> Result<(), NCError>;
    /// Every node has to send a heartbeat message to the server. If it doesn't arrive in time (2 * the heartbeat value in the NCConfiguration)
    /// then this method is called with the corresponding node id and the node should be marked as offline in this method.
    fn heartbeat_timeout(&mut self, nodes: Vec<NodeID>);
    /// When all the nodes are done with processing and all internal threads are also finished then this method is called.
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

    /// This is the main method that you call when you start the server. It expects your custom data structure that implements the NCServer trait.
    pub fn start<T: NCServer + Send + 'static>(&mut self, nc_server: T) -> Result<(), NCError> {
        debug!("NCServerStarter::new()");

        // let time_start = Instant::now();
        let server_process = Arc::new(NCServerProcess::new(&self.config, nc_server));
        let server_heartbeat = NCServerHeartbeat::new(&self.config);
        let thread_pool = ThreadPool::new((self.config.pool_size + 1) as usize);

        self.start_heartbeat_thread(&thread_pool, server_heartbeat);
        self.start_main_loop(&thread_pool, server_process.clone());

        // let time_taken = (Instant::now() - time_start).as_secs_f64();
        let time_taken = server_process.calc_total_time();

        info!("Time taken: {} s, {} min, {} h", time_taken, time_taken / 60.0, time_taken / (60.0 * 60.0));

        thread_pool.join();

        Ok(())
    }

    /// The heartbeat check thread is started here in an endless loop.
    /// It calls the method send_check_heartbeat_message() which sends the NCNodeMessage::CheckHeartbeat message
    /// to the server. The server then checks all the nodes to see if one of them missed a heartbeat.
    /// If there is an IO error the loop exits because the server also has finished its main loop and
    /// doesn't accept any tcp connections anymore.
    /// The job is done and no more heartbeats will arrive.
    fn start_heartbeat_thread(&mut self, thread_pool: &ThreadPool, server_heartbeat: NCServerHeartbeat) {
        debug!("NCServerStarter::start_heartbeat_thread()");

        thread_pool.execute(move || {
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
    /// For every node connection the method start_node_thread() is called, which handles the node request in a separate thread.
    /// If the job is done one the main loop will exited
    fn start_main_loop<T: NCServer + Send + 'static>(&self, thread_pool: &ThreadPool, server_process: Arc<NCServerProcess<T, T::CustomMessageT>>) {
        debug!("NCServerStarter::start_main_loop()");

        let ip_addr: IpAddr = "0.0.0.0".parse().unwrap(); // TODO: Make this configurable ?
        let socket_addr = SocketAddr::new(ip_addr, server_process.port);
        let listener = TcpListener::bind(socket_addr).unwrap();

        loop {
            match listener.accept() {
                Ok((stream, addr)) => {
                    debug!("Connection from node: {}", addr);
                    self.start_node_thread(thread_pool, stream, server_process.clone());
                }
                Err(e) => {
                    error!("IO error while accepting node connections: {}", e);
                }
            }

            if server_process.is_job_done() {
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
    /// This starts a new thread for each node that sends a message to the server and calls the handle_node() method in that thread.
    fn start_node_thread<T: NCServer + Send + 'static>(&self, thread_pool: &ThreadPool, stream: TcpStream, server_process: Arc<NCServerProcess<T, T::CustomMessageT>>) {
        debug!("NCServerStarter::start_node_thread()");

        thread_pool.execute(move || {
            if let Err(e) = server_process.handle_node(stream) {
                error!("Error in handle_node(): {}", e);
            }
        });
    }
}

/// Takes care of all the heartbeat time stamps for all the registered nodes.
struct NCServerHeartbeat {
    /// The socket for the server itself.
    server_socket: SocketAddr,
    /// heartbeat timeout duration * 2, this gives the node enough time to send their heartbeat messages.
    duration: Duration,
    /// Handles all the communication
    nc_communicator: Mutex<NCCommunicator>,
}

impl NCServerHeartbeat {
    /// Creates a new ServerHeartbeat with the given configuration.
    fn new(config: &NCConfiguration) -> Self {
        debug!("ServerHeartbeat::new()");

        let ip_addr: IpAddr = "127.0.0.1".parse().unwrap();
        let server_socket = SocketAddr::new(ip_addr, config.port);
        let duration = Duration::from_secs(2 * config.heartbeat);

        NCServerHeartbeat{
            server_socket,
            duration,
            nc_communicator: Mutex::new(NCCommunicator::new(config)),
        }
    }

    /// The current thread sleeps for the configured amount of time:
    /// 2 * heartbeat
    fn sleep(&self) {
        debug!("ServerHeartbeat::sleep()");

        thread::sleep(self.duration);
    }

    /// Sends the NCNodeMessage::CheckHeartbeat message to itself, so that the server
    /// can check all the registered nodes.
    fn send_check_heartbeat_message(&self) -> Result<(), NCError> {
        debug!("ServerHeartbeat::send_check_heartbeat_message()");
        let message: NCNodeMessage<(), ()> = NCNodeMessage::CheckHeartbeat;

        self.nc_communicator.lock()?.nc_send_data(&message, &self.server_socket)
    }
}

/// Some statistics about the server.
/// This is the data that will be send when a
/// NCNodeMessage::GetStatistics message arrived.
/// More items may be added in the future.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NCServerStatistics {
    /// Total number of nodes, includes inactive nodes
    num_of_nodes: usize,
    /// total time from start of server as secs
    time_taken: f64,
    /// Node ids and time since last heartbeat as secs
    hb_time_stamps: Vec<(NodeID, f64)>,
}

/// In here the server handles all the messages and generates appropriate responses.
struct NCServerProcess<T, U> {
    /// The port the server will listen to.
    port: u16,
    /// Every n seconds a heartbeat message is sent from the node to the server.
    heartbeat: u64,
    /// Time instance when the server was created
    time_start: Instant,
    /// Indicates if the job is already done and the server can exit its main loop.
    job_done: AtomicBool,
    /// The user defined data structure that implements the NCServer trait.
    nc_server: Mutex<T>,
    /// Internal list of all the registered nodes.
    node_list: Mutex<NCNodeList<U>>,
    /// Optional setting if nodes have to move to a new server
    new_server: Mutex<Option<(String, u16)>>,
    /// Handles all the communication
    nc_communicator: Mutex<NCCommunicator>,
}

impl<T: NCServer> NCServerProcess<T, T::CustomMessageT> {
    /// Creates a new ServerProcess with the given user defined nc_server that implements the NCServer trait
    fn new(config: &NCConfiguration, nc_server: T) -> Self {
        debug!("ServerProcess::new()");

        NCServerProcess{
            port: config.port,
            heartbeat: config.heartbeat,
            time_start: Instant::now(),
            job_done: AtomicBool::new(false),
            nc_server: Mutex::new(nc_server),
            node_list: Mutex::new(NCNodeList::new()),
            new_server: Mutex::new(None),
            nc_communicator: Mutex::new(NCCommunicator::new(config)),
        }
    }

    /// Returns true if the job is finished
    fn is_job_done(&self) -> bool {
        debug!("ServerProcess::is_job_done()");

        self.job_done.load(Ordering::Relaxed)
    }

    /// Returns the total time the server has been running
    fn calc_total_time(&self) -> f64 {
        self.time_start.elapsed().as_secs_f64()
    }

    /// Shut down the server gracefully when the job is done or
    /// when it is requested by the message NCNodeMessage::ShotDown
    fn shut_down(&self) {
        self.job_done.store(true, Ordering::Relaxed);
    }

    /// All the message that were sent from a node are handled here. It can be on of these types:
    /// - NCNodeMessage::Register: every new node has to register first, the server then assigns a new node id and sends some optional initial data back to the node with the
    ///   NCServerMessage::InitialData message. The server trait method initial_data() is called here.
    /// - NCNodeMessage::NeedsData: the node needs some data to process and depending on the job state the server answers this request with a NCServerMessage::JobStatus message.
    ///   The server trait method prepare_data_for_node() is called here.
    /// - NCNodeMessage::HeartBeat: the node sends a heartbeat message and the server updates the internal node list with the corresponding current time stamp.
    /// - NCNodeMessage::HasData: the node has finished processing the data and has sent the result back to the server.
    ///   The server trait method process_data_from_node() is called here.
    /// - NCNodeMessage::CheckHeartbeat: This message is sent from the check heartbeat thread to the server
    ///   itself. All the nodes will be checked for the heartbeat time stamp and if a node missed it, the NCServer trait
    ///   method heartbeat_timeout() is called where the node should be marked as offline.
    fn handle_node(&self, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::handle_node()");

        let request: NCNodeMessage<T::ProcessedDataT, T::CustomMessageT> =
            self.nc_communicator.lock()?.nc_receive_data(&mut stream)?;

        match request {
            NCNodeMessage::Register => {
                let mut node_list = self.node_list.lock()?;
                let mut nc_server = self.nc_server.lock()?;

                let node_id = node_list.register_new_node();
                let initial_data = nc_server.initial_data()?;
                info!("Registering new node: {}, {}", node_id, stream.peer_addr()?);
                self.send_initial_data_message(node_id, initial_data, stream)?;
            }
            NCNodeMessage::NeedsData(node_id) => {
                debug!("Node {} needs data to process", node_id);
                let new_server = self.new_server.lock()?;
                let mut node_list = self.node_list.lock()?;
                let mut nc_server = self.nc_server.lock()?;

                if let Some((server, port)) = new_server.clone() {
                    self.node_list.lock()?.remove_node(node_id);
                    return self.send_new_server_message(server, port, stream)
                }

                if let Some(custom_message) = node_list.get_message(node_id) {
                    debug!("Send custom message to node: {}", node_id);
                    return self.send_custom_message(custom_message, stream)
                }

                let data_for_node = nc_server.prepare_data_for_node(node_id)?;

                match data_for_node {
                    NCJobStatus::Unfinished(data) => {
                        debug!("Send data to node");
                        self.send_job_status_unfinished(data, stream)?;
                    }
                    NCJobStatus::Waiting => {
                        debug!("Waiting for other nodes to finish");
                        self.send_job_status_waiting(stream)?;
                    }
                    NCJobStatus::Finished => {
                        debug!("Job is done, will exit handle_node()");
                        // Do not bother sending a message to the nodes, they will quit anyways after the retry counter is zero.
                        // The counter will be decremented if there is an IO error.
                        // Same for the server heartbeat thread, it will exit its loop if there is an IO error.
                        self.shut_down();
                    }
                }
            }
            NCNodeMessage::HeartBeat(node_id) => {
                debug!("Got heartbeat from node: {}", node_id);
                let mut node_list = self.node_list.lock()?;

                node_list.update_heartbeat(node_id);
            }
            NCNodeMessage::HasData(node_id, data) => {
                debug!("Node {} has processed some data and we received the results", node_id);
                let mut nc_server = self.nc_server.lock()?;

                nc_server.process_data_from_node(node_id, &data)?;
            }
            NCNodeMessage::CheckHeartbeat => {
                debug!("Message CheckHeartbeat received!");
                // Check the heartbeat for all the nodes and call the trait method heartbeat_timeout()
                // with those nodes to react accordingly.
                let node_list = self.node_list.lock()?;
                let mut nc_server = self.nc_server.lock()?;

                let nodes = node_list.check_heartbeat(self.heartbeat).collect::<Vec<NodeID>>();
                nc_server.heartbeat_timeout(nodes);
            }
            NCNodeMessage::GetStatistics => {
                debug!("Statistics requested");
                // Gather some statistics and send it to the node that requested it
                let node_list = self.node_list.lock()?;

                let num_of_nodes = node_list.len();
                let time_taken = self.calc_total_time();
                let hb_time_stamps = node_list.get_time_stamps();

                let server_statistics = NCServerStatistics{
                    num_of_nodes,
                    time_taken,
                    hb_time_stamps,
                };

                self.send_server_statistics(server_statistics, stream)?;
            }
            NCNodeMessage::ShutDown => {
                debug!("Shut down requested");
                // Shut down server gracefully
                self.shut_down();
            }
            NCNodeMessage::NewServer(server, port) => {
                debug!("Move all nodes to a new server, address: {}, port: {}", server, port);
                let mut new_server = self.new_server.lock()?;

                *new_server = Some((server, port));
            }
            NCNodeMessage::NodeMigrated(node_id) => {
                debug!("Register migrated node: {}", node_id);
                let mut node_list = self.node_list.lock()?;

                node_list.migrate_node(node_id);
            }
            NCNodeMessage::CustomMessage(message, destination) => {
                let mut node_list = self.node_list.lock()?;

                match destination {
                    Some(node_id) => {
                        debug!("Add a custom message to node: {}", node_id);
                        node_list.add_message(message, node_id);
                    }
                    None => {
                        debug!("Add a custom message to all nodes");
                        node_list.add_message_all(message);
                    }
                }
            }
        }
        Ok(())
    }

    /// Sends the NCServerMessage::InitialData message to the node with the given node_id and optional initial_data
    fn send_initial_data_message(&self, node_id: NodeID, initial_data: Option<T::InitialDataT>, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::send_initial_data_message()");
        let message: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = NCServerMessage::InitialData(node_id, initial_data);

        self.nc_communicator.lock()?.nc_send_data2(&message, &mut stream)
    }

    /// Sends the NCServerMessage::JobStatus Unfinished message with the given data to the node,
    fn send_job_status_unfinished(&self, data: T::NewDataT, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::send_job_status_unfinished_message()");
        let message: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = NCServerMessage::JobStatus(NCJobStatus::Unfinished(data));

        self.nc_communicator.lock()?.nc_send_data2(&message, &mut stream)
    }

    /// Send the NCServerMessage::JobStatus Waiting message to the node.
    fn send_job_status_waiting(&self, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::send_job_status_waiting()");
        let message: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = NCServerMessage::JobStatus(NCJobStatus::Waiting);

        self.nc_communicator.lock()?.nc_send_data2(&message, &mut stream)
    }

    /// Send the NCServerMessage::Statistics to the node.
    fn send_server_statistics(&self, server_statistics: NCServerStatistics, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::send_server_statistics()");
        let message: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = NCServerMessage::Statistics(server_statistics);

        self.nc_communicator.lock()?.nc_send_data2(&message, &mut stream)
    }

    /// Send the NCServerMessage::NewServer message to the node.
    fn send_new_server_message(&self, server: String, port: u16, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::send_new_server_message()");
        let message: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = NCServerMessage::NewServer(server, port);

        self.nc_communicator.lock()?.nc_send_data2(&message, &mut stream)
    }

    /// Send the NCServerMessage::Command message to the node.
    fn send_custom_message(&self, custom_message: T::CustomMessageT, mut stream: TcpStream) -> Result<(), NCError> {
        debug!("ServerProcess::send_custom_message()");
        let message: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = NCServerMessage::CustomMessage(custom_message);

        self.nc_communicator.lock()?.nc_send_data2(&message, &mut stream)
    }
}
