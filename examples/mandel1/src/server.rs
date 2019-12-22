use std::error;

use log::{info, error};

use node_crunch::{NC_Server, nc_start_server, NC_Configuration, NC_JobStatus};

use crate::Mandel1Opt;

struct MandelServer {

}

impl NC_Server for MandelServer {
    fn prepare_data_for_node(&mut self, node_id: u128) -> Result<Vec<u8>, Box<dyn error::Error + Send>> {
        let mut result = Vec::new();

        // TODO:

        Ok(result)
    }

    fn process_data_from_node(&mut self, node_id: u128, data: &Vec<u8>) -> Result<(), Box<dyn error::Error + Send>> {
        // TODO:

        Ok(())
    }

    fn job_status(&self) -> NC_JobStatus {
        // TODO:
        NC_JobStatus::Unfinished
    }

    fn heartbeat_timeout(&mut self, node_id: u128) {
        // TODO:
    }
}

pub async fn run_server(options: Mandel1Opt) {
    let configuration = NC_Configuration {
        port: options.port,
        ..Default::default()
    };

    let node = MandelServer{};

    match nc_start_server(node, configuration).await {
        Ok(_) => {
            info!("Calculation finished");
        }
        Err(e) => {
            error!("An error occurred: {}", e);
        }
    }
}
