use log::{info, error};
use num::{complex::Complex64};
// use serde::{Serialize, Deserialize};

use node_crunch::{NCNode, NCError, nc_start_node, NCConfiguration, nc_encode_data, nc_decode_data};

use crate::{Mandel1Opt, ServerData, NodeData};

struct MandelNode {

}

fn handle_data(data: Vec<u8>) -> Result<Vec<u8>, NCError> {
    let input: ServerData = nc_decode_data(&data)?;
    let mut line: Vec<u32> = vec![0; input.img_size as usize];
    let mut c: Complex64;
    let mut z: Complex64;
    let mut iter: u32;

    let im: f64 = input.im + ((input.y as f64) * input.y_step);

    for x in 0..input.img_size {
        iter = 0;
        c = Complex64 {re: input.re + ((x as f64) * input.x_step), im: im};
        z = c;
        while (z.norm_sqr() <= 4.0) && (iter < input.max_iter) {
            z = c + (z * z);
            iter = iter + 1;
        }
        line[x as usize] = iter;
    }

    let result = nc_encode_data(&NodeData {y: input.y, line: line})?;

    Ok(result)
}

impl NCNode for MandelNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Vec<u8> {
        match handle_data(data) {
            Ok(result) => result,
            Err(e) => {
                error!("An error occurred while processing the data from the server: {}", e);
                Vec::new()
            }
        }
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
