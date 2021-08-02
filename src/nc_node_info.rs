//! This module contains the node id and node info data structure.
//! NodeID is just a new type pattern for a integer number.
//! NCNodeInfo holds the node id and a time stamp for the heartbeat.

use std::time::Instant;
use std::fmt;

use rand::random;
use serde::{Serialize, Deserialize};

/// New type pattern for the node id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeID(u64);

/// This data structure contains the node id and the time stamps for the heartbeat.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct NCNodeInfo {
    /// The id of the node.
    pub(crate) node_id: NodeID,
    /// Time stamp for the node id since the last valid heartbeat.
    pub(crate) instant: Instant,
}

pub(crate) struct NCNodeList {
    /// List of all nodes that have been registered.
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

    /// All the registered nodes are checked here. If the heartbeat time stamp
    /// is too old (> 2 * heartbeat in [`NCConfiguration`](crate::nc_config::NCConfiguration)) then
    /// the NCServer trait method [`heartbeat_timeout()`](crate::nc_server::NCServer::heartbeat_timeout)
    /// is called where the node should be marked as offline.
    pub(crate) fn check_heartbeat(&self, heartbeat_duration: u64) -> impl Iterator<Item=NodeID> + '_ {
        self.nodes.iter().filter(move |node| node.heartbeat_invalid(heartbeat_duration)).map(|node| node.node_id)
    }

    /// This method generates a new and unique node id for a new node that has just registered with the server.
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

    /// Update the heartbeat timestamp for the given node.
    /// This happens when the heartbeat thread in the [`nc_node`](crate::nc_node) module
    /// has send the [`NCNodeMessage::HeartBeat`](crate::nc_node::NCNodeMessage) message to the server.
    pub(crate) fn update_heartbeat(&mut self, node_id: NodeID) {
        for node in self.nodes.iter_mut() {
            if node.node_id == node_id {
                node.update_heartbeat();
                break
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_heartbeat_invalid() {
        let node_info = NCNodeInfo::new(NodeID::unset());

        thread::sleep(Duration::from_secs(3));

        assert!(!node_info.heartbeat_invalid(5));

        thread::sleep(Duration::from_secs(3));

        assert!(node_info.heartbeat_invalid(5));
    }

    #[test]
    fn test_update_heartbeat() {
        let mut node_info = NCNodeInfo::new(NodeID::unset());

        thread::sleep(Duration::from_secs(5));

        assert!(node_info.heartbeat_invalid(3));

        node_info.update_heartbeat();

        assert!(!node_info.heartbeat_invalid(3));
    }

    #[test]
    fn test_register_new_node() {
        let mut node_list = NCNodeList::new();

        let node = node_list.register_new_node();

        assert_eq!(node_list.nodes.len(), 1);

        assert_eq!(node_list.nodes[0].node_id, node);

        let node = node_list.register_new_node();

        assert_eq!(node_list.nodes.len(), 2);

        assert_ne!(node_list.nodes[0].node_id, node);
        assert_eq!(node_list.nodes[1].node_id, node);
    }

    #[test]
    fn test_node_list_check_heartbeat() {
        let mut node_list = NCNodeList::new();

        let _ = node_list.register_new_node();
        let _ = node_list.register_new_node();
        let _ = node_list.register_new_node();
        let _ = node_list.register_new_node();

        let result = node_list.check_heartbeat(5);
        let result = result.collect::<Vec<NodeID>>();

        assert_eq!(result.len(), 0);

        thread::sleep(Duration::from_secs(5));

        let result = node_list.check_heartbeat(3);
        let result = result.collect::<Vec<NodeID>>();

        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_node_list_update_heartbeat() {
        let mut node_list = NCNodeList::new();

        let _ = node_list.register_new_node();
        let _ = node_list.register_new_node();
        let node_id = node_list.register_new_node();
        let _ = node_list.register_new_node();

        thread::sleep(Duration::from_secs(5));

        node_list.update_heartbeat(node_id);

        let result = node_list.check_heartbeat(3);
        let result = result.collect::<Vec<NodeID>>();

        assert_eq!(result.len(), 3);

        for other_id in result {
            assert_ne!(other_id, node_id);
        }
    }
}
