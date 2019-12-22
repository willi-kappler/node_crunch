use std::error;

use log::{info, error};

use node_crunch::{NC_Node, nc_start_node, NC_Configuration};

use crate::Mandel1Opt;

struct MandelNode {

}

impl NC_Node for MandelNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Result<Vec<u8>, Box<dyn error::Error + Send>> {
        let mut result = Vec::new();

        // TODO: do calculation

        Ok(result)
    }
}

pub async fn run_node(options: Mandel1Opt) {
    let configuration = NC_Configuration {
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
