//! This module contains the node id and node info data structure.
//! NodeID is just a newtype pattern for a integer number.
//! NCNodeInfo holds the node id and a time stamp for the heartbeat.

use std::time::Instant;
use std::fmt;

use rand::random;
use serde::{Serialize, Deserialize};

/// Newtype pattern for the node id.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeID(u64);

/// This data strutcure contains the node id and the time stamps for the heartbeat.
pub(crate) struct NCNodeInfo {
    pub(crate) node_id: NodeID,
    pub(crate) instant: Instant,
}

impl NodeID {
    /*
    pub fn new() -> Self {
        NodeID(0)
    }
    */

    /// Create a new random node id.
    pub fn random() -> Self {
        NodeID(random())
    }
}

impl fmt::Display for NodeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl NCNodeInfo {
    /// Create a new node info with the given node id and the current time stamp.
    pub fn new(node_id: NodeID) -> Self {
        NCNodeInfo{ node_id, instant: Instant::now() }
    }

    /// When a node sends a valid heartbeat update the time stamp for that node.
    pub fn update_heartbeat(&mut self) {
        self.instant = Instant::now();
    }

    /// Check if the heartbeat for the node is invalid using the given time range.
    pub fn heartbeat_invalid(&self, limit: u64) -> bool {
        let diff = Instant::now() - self.instant;
        diff.as_secs() > limit
    }
}
