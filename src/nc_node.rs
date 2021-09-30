//! This module contains the nc node message, trait and helper methods.
//! To use the node you have to implement the NCNode trait that has two methods:
//! set_initial_data() and process_data_from_server()

use std::net::{IpAddr, SocketAddr};
use std::{thread, time::Duration};
use std::thread::{spawn, JoinHandle};

use log::{error, info, debug};
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::nc_error::NCError;
use crate::nc_server::{NCServerMessage, NCJobStatus};
use crate::nc_config::NCConfiguration;
use crate::nc_node_info::NodeID;
use crate::nc_communicator::{NCCommunicator};

/// This message is sent from the node to the server in order to register, receive new data and send processed data.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCNodeMessage<ProcessedDataT, CustomMessageT> {
    /// Register this node with the server. The server will assign a new node id to this node and answers with a NCServerMessage::InitialData message.
    /// This is the first thing every node has to do!
    Register,
    /// This node needs new data to process. The server answers with a JobStatus message.
    NeedsData(NodeID),
    /// This node has finished processing the data and sends it to the server. No answer from the server.
    HasData(NodeID, ProcessedDataT),
    /// This node sends a heartbeat message every n seconds. The time span between two heartbeats is set in the configuration NCConfiguration.
    HeartBeat(NodeID),
    /// This is a message that the server sends to itself to break out from blocking on node connection via accept() and
    /// start checking the heartbeat time stamps of all nodes.
    CheckHeartbeat,
    /// Get some statistics from the server:
    /// - number of active nodes (node ids)
    /// - other items
    GetStatistics,
    /// Tell the server to shut down
    ShutDown,
    /// Move all the nodes to a new server given by address and port
    MoveServer(String, u16),
    /// Send a custom message to one or all nodes
    CustomMessage(CustomMessageT, Option<NodeID>),
    // More items may be added in the future
}

/// This trait has to be implemented for the code that runs on all the nodes.
pub trait NCNode {
    type InitialDataT: Serialize + DeserializeOwned;
    type NewDataT: Serialize + DeserializeOwned;
    type ProcessedDataT: Serialize + DeserializeOwned;
    type CustomMessageT: Serialize + DeserializeOwned;

    /// Once this node has sent a NCNodeMessage::Register message the server responds with a NCServerMessage::InitialData message.
    /// Then this method is called with the data received from the server.
    fn set_initial_data(&mut self, node_id: NodeID, initial_data: Option<Self::InitialDataT>) -> Result<(), NCError> {
        debug!("Got new node id: {}", node_id);

        match initial_data {
            Some(_) => debug!("Got some initial data from the server."),
            None => debug!("Got no initial data from the server.")
        }

        Ok(())
    }

    /// Whenever the node requests new data from the server, the server will respond with new data that needs to be processed by the node.
    /// This method is then called with the data that was received from the server.
    /// Here you put your code that does the main number crunching on every node.
    /// Note that you have to use the nc_decode_data() or nc_decode_data2() helper methods from the nc_utils module in order to
    /// deserialize the data.
    fn process_data_from_server(&mut self, data: &Self::NewDataT) -> Result<Self::ProcessedDataT, NCError>;

    /// The server has send a special user defined command to the node.
    /// Usually this is not needed, only for debug purposes or if s.th. special has happened (user interaction for example)
    fn process_custom_message(&mut self, _command: &Self::CustomMessageT) {
        debug!("Got a command from server");
    }
}

/// Main data structure for managing and starting the computation on the nodes.
pub struct NCNodeStarter {
    /// Configuration for the server and the node.
    config: NCConfiguration,
}

impl NCNodeStarter {
    /// Create a new NCNodeStarter using the given configuration
    pub fn new(config: NCConfiguration) -> Self {
        debug!("NCNodeStarter::new()");

        NCNodeStarter{ config }
    }

    /// The main entry point for the code that runs on all nodes.
    /// You give it your own user defined data structure that implements the NCNode trait.
    /// Everything else is done automatically for you.
    /// The NCNode trait method set_initial_data() is called here once in order to set the node id and some optional data that is
    /// send to all nodes at the beginning.
    pub fn start<T: NCNode>(&mut self, nc_node: T) -> Result<(), NCError> {
        debug!("NCNodeStarter::start()");

        let ip_addr: IpAddr = self.config.address.parse()?;
        let server_addr = SocketAddr::new(ip_addr, self.config.port);

        let mut node_process = NodeProcess::new(server_addr, nc_node, &self.config);
        node_process.get_initial_data()?;

        let node_heartbeat = NodeHeartbeat::new(server_addr, node_process.node_id, &self.config);

        let thread_handle = self.start_heartbeat_thread(node_heartbeat);
        self.start_main_loop(node_process);
        thread_handle.join().unwrap();

        info!("Job done, exit now");
        Ok(())
    }

    /// The heartbeat thread that runs in the background and sends heartbeat messages to the server is started here.
    /// It does this every n seconds which can be configured in the NCConfiguration data structure.
    /// If the server doesn't receive the heartbeat within the valid time span, the server marks the node internally as offline
    /// and gives another node the same data chunk to process.
    fn start_heartbeat_thread(&self, mut node_heartbeat: NodeHeartbeat) -> JoinHandle<()> {
        debug!("NCNodeStarter::start_heartbeat_thread()");

        spawn(move || {
            loop {
                node_heartbeat.sleep();

                if let Err(e) = node_heartbeat.send_heartbeat_message() {
                    error!("Error in send_heartbeat(): {}, retry_counter: {}", e, node_heartbeat.get_counter());

                    if node_heartbeat.dec_and_check_counter() {
                        debug!("Retry counter is zero, will exit now");
                        break
                    }

                } else {
                    // Reset the counter if message was sent successfully
                    node_heartbeat.reset_counter();
                }
            }

            debug!("Heartbeat loop finished")
        })
    }

    /// Here is main loop for this node. It keeps requesting and processing data until the server
    /// is finished. There will be no finish message from the server and the node will just run into
    /// a timeout and exit.
    /// If there is an error this node will wait n seconds before it tries to reconnect to the server.
    /// The delay time can be configured in the NCConfiguration data structure.
    /// With every error the retry counter is decremented. If it reaches zero the node will give up and exit.
    /// The counter can be configured in the NCConfiguration.
    fn start_main_loop<T: NCNode>(&self, mut node_process: NodeProcess<T>) {
        debug!("NCNodeStarter::start_main_loop()");

        loop {
            debug!("Ask server for new data");

            if let Err(e) = node_process.get_and_process_data() {
                error!("Error in get_and_process_data(): {}, retry counter: {:?}", e, node_process.get_counter());

                if node_process.dec_and_check_counter() {
                    debug!("Retry counter is zero, will exit now");
                    break
                }

                debug!("Will wait before retry (delay_request_data: {} sec)", node_process.get_delay());
                node_process.sleep();
            } else {
                // Reset the counter if message was sent successfully
                node_process.reset_counter()
            }
        }

        debug!("Main loop finished")
    }
}

/// Manages and sends heartbeat messages to the server.
struct NodeHeartbeat {
    /// IP address and port of the server.
    server_addr: SocketAddr,
    /// The node id for this node,
    node_id: NodeID,
    /// How often should the heartbeat thread try to contact the server before giving up.
    retry_counter: RetryCounter,
    /// Send every heartbeat_duration seconds the xxx message to the server.
    heartbeat_duration: Duration,
    /// Handles all the communication
    nc_communicator: NCCommunicator,
}

impl NodeHeartbeat {
    /// Creates a new NodeHeartbeat with the given arguments.
    fn new(server_addr: SocketAddr, node_id: NodeID, config: &NCConfiguration) -> Self {
        debug!("NodeHeartbeat::new()");

        NodeHeartbeat {
            server_addr,
            node_id,
            retry_counter: RetryCounter::new(config.retry_counter),
            heartbeat_duration: Duration::from_secs(config.heartbeat),
            nc_communicator: NCCommunicator::new(config),
        }
    }

    /// The heartbeat thread will sleep for the given duration from the configuration.
    fn sleep(&self) {
        debug!("NodeHeartbeat::sleep()");

        thread::sleep(self.heartbeat_duration);
    }

    /// Send the NCNodeMessage::HeartBeat message to the server.
    fn send_heartbeat_message(&mut self) -> Result<(), NCError> {
        debug!("NodeHeartbeat::send_heartbeat_message()");
        let message: NCNodeMessage<(), ()> = NCNodeMessage::HeartBeat(self.node_id);

        self.nc_communicator.nc_send_data(&message, &self.server_addr)
    }

    /// Returns the current value of the retry counter.
    fn get_counter(&self) -> u64 {
        debug!("NodeHeartbeat::get_counter()");

        self.retry_counter.counter
    }

    /// Decrement the retry counter on error and check if it is zero.
    /// If zero return true, else false.
    fn dec_and_check_counter(&mut self) -> bool {
        debug!("NodeHeartbeat::dec_and_check_counter()");

        self.retry_counter.dec_and_check()
    }

    /// Resets the retry counter to the initial value when there was no error.
    fn reset_counter(&mut self) {
        debug!("NodeHeartbeat::reset_counter()");

        self.retry_counter.reset()
    }
}

/// Communication with the server and processing of data.
struct NodeProcess<T> {
    /// IP address and port of the server.
    server_addr: SocketAddr,
    /// The suer defined data structure that implements the NCNode trait.
    nc_node: T,
    /// The node id for this node,
    node_id: NodeID,
    /// How often should the main processing loop try to contact the server before giving up.
    retry_counter: RetryCounter,
    /// In case of IO error wait delay_duration seconds before trying to contact the server again.
    delay_duration: Duration,
    /// Handles all the communication
    nc_communicator: NCCommunicator,
}

impl<T: NCNode> NodeProcess<T> {
    /// Creates a new NodeProcess with the given arguments.
    fn new(server_addr: SocketAddr, nc_node: T, config: &NCConfiguration) -> Self {
        debug!("NodeProcess::new()");

        NodeProcess{
            server_addr,
            nc_node,
            // This will be set in the method get_initial_data()
            node_id: NodeID::unset(),
            retry_counter: RetryCounter::new(config.retry_counter),
            delay_duration: Duration::from_secs(config.delay_request_data),
            nc_communicator: NCCommunicator::new(config),
        }
    }

    /// This is called once at the beginning of NCNodeStarter::start().
    /// It sends a NCNodeMessage::Register message to the server and expects a NCServerMessage::InitialData message from the server.
    /// On success it sets the new assigned node id for this node and calls the NCNode trait method set_initial_data().
    /// If the server doesn't respond with a NCServerMessage::InitialData message a NCError::ServerMsgMismatch error is returned.
    fn get_initial_data(&mut self) -> Result<(), NCError> {
        debug!("NodeProcess::get_initial_data()");

        let initial_data: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = self.send_register_message()?;

        match initial_data {
            NCServerMessage::InitialData(node_id, initial_data) => {
                info!("Got node_id: {} and initial data from server", node_id);
                self.node_id = node_id;
                self.nc_node.set_initial_data(node_id, initial_data)
            }
            _msg => {
                error!("Error in get_initial_data(), NCServerMessage mismatch, expected: InitialData");
                Err(NCError::ServerMsgMismatch)
            }
        }
    }

    /// Send the NCNodeMessage::Register message to the server.
    fn send_register_message(&mut self) -> Result<NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT>, NCError> {
        debug!("NodeProcess::send_register_message()");
        let message: NCNodeMessage<T::ProcessedDataT, T::CustomMessageT> = NCNodeMessage::Register;

        self.nc_communicator.nc_send_receive_data(&message, &self.server_addr)
    }

    /// This method sends a NCNodeMessage::NeedsData message to the server and reacts accordingly to the server response:
    /// Only one message is expected as a response from the server: NCServerMessage::JobStatus. This status can have two values
    /// 1. NCJobStatus::Unfinished: This means that the job is note done and there is still some more data to be processed.
    ///      This node will then process the data calling the process_data_from_server() method and sends the data back to the
    ///      server using the NCNodeMessage::HasData message.
    /// 2. NCJobStatus::Waiting: This means that not all nodes are done and the server is still waiting for all nodes to finish.
    /// If the server sends a different message this method will return a NCError::ServerMsgMismatch error.
    fn get_and_process_data(&mut self) -> Result<(), NCError> {
        debug!("NodeProcess::get_and_process_data()");

        let new_data: NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT> = self.send_needs_data_message()?;

        match new_data {
            NCServerMessage::JobStatus(job_status) => {
                match job_status {
                    NCJobStatus::Unfinished(data) => {
                        self.process_data_and_send_has_data_message(&data)
                    }
                    NCJobStatus::Waiting => {
                        // The node will not exit here since the job is not 100% done.
                        // This just means that all the remaining work has already
                        // been distributed among all nodes.
                        // One of the nodes can still crash and thus free nodes have to ask the server for more work
                        // from time to time (delay_request_data).

                        debug!("Waiting for other nodes to finish (delay_request_data: {} sec)...", self.get_delay());
                        self.sleep();
                        Ok(())
                    }
                    _msg => {
                        // The server does not bother sending the node a NCJobStatus::Finished message.
                        error!("Error: unexpected message from server");
                        Err(NCError::ServerMsgMismatch)
                    }
                }
            }
            NCServerMessage::CustomMessage(message) => {
                // Forward custom message to user code
                self.nc_node.process_custom_message(&message);
                Ok(())
            }
            _ => {
                error!("Error in process_data_and_send_has_data_message(), NCServerMessage mismatch");
                Err(NCError::ServerMsgMismatch)
            }
        }
    }

    /// Send the NCNodeMessage::NeedsData message to the server.
    fn send_needs_data_message(&mut self) -> Result<NCServerMessage<T::InitialDataT, T::NewDataT, T::CustomMessageT>, NCError> {
        debug!("NodeProcess::send_needs_data_message()");
        let message: NCNodeMessage<T::ProcessedDataT, T::CustomMessageT> = NCNodeMessage::NeedsData(self.node_id);

        self.nc_communicator.nc_send_receive_data(&message, &self.server_addr)
    }

    /// Process the new data from the server and sends the result back to the server using
    /// the NCNodeMessage::HasData message.
    fn process_data_and_send_has_data_message(&mut self, data: &T::NewDataT) -> Result<(), NCError> {
        debug!("NodeProcess::process_data_and_send_has_data_message()");
        let result = self.nc_node.process_data_from_server(data)?;
        let message: NCNodeMessage<T::ProcessedDataT, T::CustomMessageT> = NCNodeMessage::HasData(self.node_id, result);

        self.nc_communicator.nc_send_data(&message, &self.server_addr)
    }

    /// Returns the current value of the retry counter.
    fn get_counter(&self) -> u64 {
        debug!("NodeProcess::get_counter()");

        self.retry_counter.counter
    }

    /// Decrement the retry counter on error and check if it is zero.
    /// If zero return true, else false.
    fn dec_and_check_counter(&mut self) -> bool {
        debug!("NodeProcess::dec_and_check_counter()");

        self.retry_counter.dec_and_check()
    }

    /// Returns the delay duration in seconds.
    fn get_delay(&self) -> u64 {
        debug!("NodeProcess::get_delay()");

        self.delay_duration.as_secs()
    }

    /// The current thread in the main loop sleeps for the given delay from the configuration file.
    fn sleep(&self) {
        debug!("NodeProcess::sleep()");

        thread::sleep(self.delay_duration);
    }

    /// Resets the retry counter to the initial value when there was no error.
    fn reset_counter(&mut self) {
        debug!("NodeProcess::reset_counter()");

        self.retry_counter.reset()
    }
}

/// Counter for nc_node if connection to server is not possible.
/// The counter will be decreased every time there is an IO error and if it is zero the method dec_and_check
/// returns true, otherwise false.
/// When the connection to the server is working again, the counter is reset to its initial value.
#[derive(Debug, Clone)]
struct RetryCounter {
    /// The initial value for the counter. It can be reset to this value when a message has been send / received successfully.
    init: u64,
    /// The current value for the counter. It will be decremented in an IO error case.
    counter: u64,
}

impl RetryCounter {
    /// Create a new retry counter with the given limit.
    /// It will count backwards to zero.
    fn new(counter: u64) -> Self {
        debug!("RetryCounter::new()");

        RetryCounter{ init: counter, counter }
    }

    /// Decrements and checks the counter.
    /// If it's zero return true, else return false.
    fn dec_and_check(&mut self) -> bool {
        debug!("RetryCounter::dec_and_check()");

        if self.counter == 0 {
            true
        } else {
            self.counter -= 1;
            false
        }
    }

    /// Resets the counter to it's initial value.
    fn reset(&mut self) {
        debug!("RetryCounter::reset()");

        self.counter = self.init
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::{IpAddr, Ipv4Addr};

    struct TestNode;

    impl NCNode for TestNode {
        type InitialDataT = ();
        type NewDataT = ();
        type ProcessedDataT = ();
        type CustomMessageT = ();

        fn process_data_from_server(&mut self, _data: &()) -> Result<(), NCError> {
            Ok(())
        }
    }

    #[test]
    fn test_nhb_dec_and_check_counter1() {
        let config = NCConfiguration::default();
        let mut nhb = NodeHeartbeat::new(
    SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                8080),
        NodeID::unset(), &config);

        assert_eq!(nhb.get_counter(), 5);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 4);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 3);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 2);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 1);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 0);
        assert!(nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 0);
    }

    #[test]
    fn test_nhb_reset_counter() {
        let config = NCConfiguration::default();
        let mut nhb = NodeHeartbeat::new(
    SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                8080),
        NodeID::unset(), &config);

        assert_eq!(nhb.get_counter(), 5);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 4);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 3);
        nhb.reset_counter();
        assert_eq!(nhb.get_counter(), 5);
        assert!(!nhb.dec_and_check_counter());
        assert_eq!(nhb.get_counter(), 4);
    }

    #[test]
    fn test_np_dec_and_check_counter() {
        let nc_node = TestNode{};
        let config = NCConfiguration::default();
        let mut np = NodeProcess::new(
        SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                  8080),
            nc_node, &config);

        assert_eq!(np.get_counter(), 5);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 4);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 3);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 2);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 1);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 0);
        assert!(np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 0);
    }

    #[test]
    fn test_np_reset_counter() {
        let nc_node = TestNode{};
        let config = NCConfiguration::default();
        let mut np = NodeProcess::new(
        SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                  8080),
            nc_node, &config);

        assert_eq!(np.get_counter(), 5);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 4);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 3);
        np.reset_counter();
        assert_eq!(np.get_counter(), 5);
        assert!(!np.dec_and_check_counter());
        assert_eq!(np.get_counter(), 4);
    }
}
