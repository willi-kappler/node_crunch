use std::time::{Instant};

use message_io::network::{Endpoint};

pub(crate) struct NCNodeInfo {
    pub(crate) node_id: u64,
    pub(crate) endpoint: Endpoint,
    pub(crate) instant: Instant,
    pub(crate) hostname: String,
}

impl NCNodeInfo {
    pub fn new(node_id: u64, endpoint: Endpoint, hostname: String) -> Self {
        NCNodeInfo{ node_id, endpoint, instant: Instant::now(), hostname }
    }

    pub fn update_heartbeat(&mut self) {
        self.instant = Instant::now();
    }

    pub fn heartbeat_invalid(&self, limit: u64) -> bool {
        let diff = Instant::now() - self.instant;
        diff.as_secs() > limit
    }
}
