

#[derive(Debug, Clone)]
pub struct NCConfiguration{
    pub address: String,
    pub port: u16,
    // pub reconnect_wait: u64,
    // pub server_timeout: u64,
    pub heartbeat: u64,
    pub delay_request_data: u64,
    pub job_status: u64,
}

impl Default for NCConfiguration {
    fn default() -> Self {
        NCConfiguration {
            address: "127.0.0.1".to_string(),
            port: 9000,
            // reconnect_wait: 10,
            // server_timeout: 60 * 10,
            heartbeat: 60, // Check every n seconds
            delay_request_data: 60,
            job_status: 60, // Check every n seconds
        }
    }
}
