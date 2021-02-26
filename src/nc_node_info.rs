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

pub(crate) struct NCNodeList {
    nodes: Vec<NCNodeInfo>,
}

impl NodeID {
    /// Create a new temporary node id that will be set later
    pub(crate) fn unset() -> Self {
        NodeID(0)
    }

    /// Create a new random node id.
    pub(crate) fn random() -> Self {
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
    pub(crate) fn new(node_id: NodeID) -> Self {
        NCNodeInfo{ node_id, instant: Instant::now() }
    }

    /// When a node sends a valid heartbeat update the time stamp for that node.
    pub(crate) fn update_heartbeat(&mut self) {
        self.instant = Instant::now();
    }

    /// Check if the heartbeat for the node is invalid using the given time range.
    pub(crate) fn heartbeat_invalid(&self, limit: u64) -> bool {
        let diff = Instant::now() - self.instant;
        diff.as_secs() > limit
    }
}

impl NCNodeList {
    pub(crate) fn new() -> Self {
        NCNodeList { nodes: Vec::new() }
    }

    /// All the registered nodes are checked here. If the heartbeat time stamp is too old (> 2 * heartbeat in NCConfiguration) then
    /// the NCServer trait function heartbeat_timeout() is called where the node should be marked as offline.
    pub(crate) fn check_heartbeat(&self, heartbeat_duration: u64) -> impl Iterator<Item=NodeID> + '_ {
        self.nodes.iter().filter(move |node| node.heartbeat_invalid(heartbeat_duration)).map(|node| node.node_id)
    }

    /// This function generates a new and unique node id for a new node that has just registered with the server.
    /// It loops through the list of all nodes and checks wether the new id is already taken. If yes a new random id
    /// will be created and re-checked with the node list.
    pub(crate) fn register_new_node(&mut self) -> NodeID {
        let mut new_id: NodeID = NodeID::random();

        'l1: loop {
            for node_info in self.nodes.iter() {
                if node_info.node_id == new_id {
                    new_id = NodeID::random();
                    continue 'l1
                }
            }

            break
        }

        self.nodes.push(NCNodeInfo::new(new_id));

        new_id
    }

    /// Update the heartbeat timstamp for the given node.
    /// This happens when the heartbeat thread in the nc_node module has send the NCNodeMessage::HeartBeat message to the server.
    pub(crate) fn update_heartbeat(&mut self, node_id: NodeID) {
        for node in self.nodes.iter_mut() {
            if node.node_id == node_id {
                node.update_heartbeat();
                break
            }
        }
    }
}
