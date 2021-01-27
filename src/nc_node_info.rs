use std::time::{Instant};
use std::fmt;

use rand::{random};
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeID(u64);

pub(crate) struct NCNodeInfo {
    pub(crate) node_id: NodeID,
    pub(crate) instant: Instant,
}

impl NodeID {
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
    pub fn new(node_id: NodeID) -> Self {
        NCNodeInfo{ node_id, instant: Instant::now() }
    }

    pub fn update_heartbeat(&mut self) {
        self.instant = Instant::now();
    }

    pub fn heartbeat_invalid(&self, limit: u64) -> bool {
        let diff = Instant::now() - self.instant;
        diff.as_secs() > limit
    }
}
