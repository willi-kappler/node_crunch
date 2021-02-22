//! This module contains the configuration for the server and the nodes.
//! Usually code for the server and the node is shared.

/// This data structure contains the configuration for the server and the node.
#[derive(Debug, Clone)]
pub struct NCConfiguration{
    /// IP address of the server
    pub address: String,
    /// Port used by the server, default is 9000
    pub port: u16,
    /// How long the server should wait for a node to connect before cheking if the job is done,
    /// The server will imediatelly continue to wait for node connections after that.
    /// Timeout duration in seconds, default is 60 seconds.
    pub server_socket_timeout: u8,
    /// When the job is done, how often should the server check for node connections before exiting.
    /// Default is 5 times.
    pub finish_countdown: u8,
    /// Nodes have to send a heartbeat every n seconds or they will be marked as offline.
    /// The function heartbeat_timeout(node_id) with the coresponding node ID is called.
    /// Default is 10 seconds.
    pub heartbeat: u64,
    /// Nodes will wait n seconds before contacting the server again to prevent a denial of service.
    /// Default is 10 seconds.
    pub delay_request_data: u64,
    /// Number of times a node should try to contact the server before givin up.
    /// Default is 10 retries.
    pub retry_counter: u8,
}

impl Default for NCConfiguration {
    fn default() -> Self {
        NCConfiguration {
            address: "127.0.0.1".to_string(),
            port: 9000,
            server_socket_timeout: 10,
            finish_countdown: 5,
            heartbeat: 60,
            delay_request_data: 10,
            retry_counter: 10,
        }
    }
}
