use std::error;

use log::{info, error};

use node_crunch::{NCNode, nc_start_node, NCConfiguration};

use crate::Mandel1Opt;

struct MandelNode {

}

impl NCNode for MandelNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Result<Vec<u8>, Box<dyn error::Error + Send>> {
        let mut result = Vec::new();

        // TODO: do calculation

        Ok(result)
    }
}

pub async fn run_node(options: Mandel1Opt) {
    let configuration = NCConfiguration {
        port: options.port,
        address: options.ip,
        ..Default::default()
    };

    let node = MandelNode{};

    match nc_start_node(node, configuration).await {
        Ok(_) => {
            info!("Calculation finished");
        }
        Err(e) => {
            error!("An error occurred: {}", e);
        }
    }
}
