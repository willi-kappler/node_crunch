//! This module contains the configuration for the server and the nodes.
//! Usually code for the server and the node is shared.

/// This data structure contains the configuration for the server and the node.
#[derive(Debug, Clone)]
pub struct NCConfiguration{
    /// IP address of the server
    pub address: String,
    /// Port used by the server
    pub port: u16,
    /// Nodes have to send a heartbeat every n seconds or they will be marked as offline.
    /// The method heartbeat_timeout(node_id) with the corresponding node ID is called.
    pub heartbeat: u64,
    /// Nodes will wait n seconds before contacting the server again to prevent a denial of service.
    pub delay_request_data: u64,
    /// Number of times a node should try to contact the server before givin up.
    pub retry_counter: u64,
}

impl Default for NCConfiguration {
    fn default() -> Self {
        NCConfiguration {
            address: "127.0.0.1".to_string(),
            port: 9000,
            heartbeat: 60,
            delay_request_data: 60,
            retry_counter: 5,
        }
    }
}
