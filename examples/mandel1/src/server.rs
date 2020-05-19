use log::{info, error, debug};
use num::{complex::Complex64};
// use serde::{Serialize, Deserialize};

use node_crunch::{NCServer, nc_start_server, NCConfiguration, NCJobStatus, nc_encode_data, nc_decode_data};

use crate::{Mandel1Opt, ServerData, NodeData};

#[derive(Debug, Clone, PartialEq)]
enum MandelData {
    Empty,
    Processing(u128),
    Finished(Vec<u32>),
}

#[derive(Debug, Clone)]
struct MandelServer {
    start: Complex64,
    end: Complex64,
    x_step: f64,
    y_step: f64,
    max_iter: u32,
    img_size: u32,
    data: Vec<MandelData>,
}

impl NCServer for MandelServer {
    fn prepare_data_for_node(&mut self, node_id: u128) -> Vec<u8> {
        debug!("Server::prepare_data_for_node, node_id: {}", node_id);

        for y in 0..self.img_size {
            if self.data[y as usize] == MandelData::Empty {
                let output = ServerData {
                    img_size: self.img_size,
                    max_iter: self.max_iter,
                    x_step: self.x_step,
                    y: y,
                    y_step: self.y_step,
                    re: self.start.re,
                    im: self.start.im,
                };

                match nc_encode_data(&output) {
                    Ok(data) => {
                        self.data[y as usize] = MandelData::Processing(node_id);
                        return data
                    },
                    Err(e) => {
                        error!("An error occurred while preparing the data for the Node: {}, error: {}", node_id, e);
                    },
                }
            }
        }
        return Vec::new()
    }

    fn process_data_from_node(&mut self, node_id: u128, data: &Vec<u8>) {
        debug!("Server::process_data_from_node, node_id: {}", node_id);

        match nc_decode_data::<NodeData>(data) {
            Ok(data) => {
                if self.data[data.y as usize] == MandelData::Processing(node_id) {
                    self.data[data.y as usize] = MandelData::Finished(data.line)
                } else {
                    error!("Missmatch data, should be Processing with node_id: {}, but is {:?}", node_id, self.data[data.y as usize])
                }
            },
            Err(e) => {
                error!("An error occurred while processing the data from the Node: {}", e)
            }
        }
    }

    fn job_status(&self) -> NCJobStatus {
        let mut empty = 0;
        let mut processing = 0;
        let mut finished = 0;

        for d in &self.data {
            match d {
                MandelData::Empty => empty = empty + 1,
                MandelData::Processing(_) => processing = processing + 1,
                MandelData::Finished(_) => finished = finished + 1,
            }
        }

        debug!("Job status: empty: {}, processing: {}, finished: {}", empty, processing, finished);

        if empty > 0 {
            NCJobStatus::Unfinished
        } else if processing > 0 {
            NCJobStatus::Waiting
        } else {

            NCJobStatus::Finished
        }
    }

    fn heartbeat_timeout(&mut self, node_id: u128) {
        info!("Heartbeat timeout, node_id: {}", node_id);

        for i in 0..self.img_size {
            if self.data[i as usize] == MandelData::Processing(node_id) {
                // Since the node is no longer responding, set data[i] as empty for other nodes to process
                self.data[i as usize] = MandelData::Empty
            }
        }
    }
}

pub async fn run_server(options: Mandel1Opt) {
    let configuration = NCConfiguration {
        port: options.port,
        ..Default::default()
    };

    let start = Complex64{re: -2.0, im: -1.5};
    let end = Complex64{re: 1.0, im: 1.5};
    let img_size = 4096;
    let max_iter = 4096;
    let x_step = (end.re - start.re) / (img_size as f64);
    let y_step = (end.im - start.im) / (img_size as f64);

    let node = MandelServer {
        start,
        end,
        x_step,
        y_step,
        max_iter,
        img_size,
        data: vec![MandelData::Empty; img_size as usize],
    };

    match nc_start_server(node, configuration).await {
        Ok(_) => {
            info!("Calculation finished");
        }
        Err(e) => {
            error!("An error occurred: {}", e);
        }
    }
}
