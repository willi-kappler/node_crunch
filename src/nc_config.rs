

#[derive(Debug, Clone)]
pub struct NC_Configuration{
    pub address: String,
    pub port: u16,
    pub reconnect_wait: u64,
    // pub heartbeat_timeout: u32,
}

impl Default for NC_Configuration {
    fn default() -> Self {
        NC_Configuration {
            address: "127.0.0.1".to_string(),
            port: 9000,
            reconnect_wait: 10,
        }
    }
}
