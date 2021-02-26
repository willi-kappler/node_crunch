use log::{info, error};
use num::complex::Complex64;

use node_crunch::{NCNode, NCError, NCConfiguration, Array2D,
    NCNodeStarter, nc_decode_data, nc_encode_data};

use crate::{Mandel1Opt, ServerData, NodeData};

/// In this example the NCNode data struct has no useful data, just code.
struct MandelNode {
}

impl NCNode for MandelNode {
    /// This processes the data that has been send from the server to this node.
    /// In here the whole number crunshing is happending in this example the mandelbrot set.
    /// The result is returned in a Ok(Vec<u8>).
    /// Return an error otherwise.
    fn process_data_from_server(&mut self, data: &[u8]) -> Result<Vec<u8>, NCError> {
        let input: ServerData = nc_decode_data(&data)?;
        let mut array2d = Array2D::<u32>::new(input.width, input.height, 0);
        let mut c: Complex64;
        let mut z: Complex64;
        let mut iter: u32;

        for x in 0..input.width {
            for y in 0..input.height {
                iter = 0;
                let re = input.re + (((x + input.x) as f64) * input.x_step);
                let im = input.im + (((y + input.y) as f64) * input.y_step);
                c = Complex64 { re, im };
                z = c;
                while (z.norm_sqr() <= 4.0) && (iter < input.max_iter) {
                    z = c + (z * z);
                    iter = iter + 1;
                }
                array2d.set(x, y, iter);
            }
        }

        let result = nc_encode_data(&NodeData { chunk_id: input.chunk_id, source: array2d })?;
        Ok(result)
    }
}

/// Starts the node with the given configuration.
pub fn run_node(options: Mandel1Opt) {
    let configuration = NCConfiguration {
        port: options.port,
        address: options.ip,
        ..Default::default()
    };

    let node = MandelNode{};
    let mut node_starter = NCNodeStarter::new(configuration);

    match node_starter.start(node) {
        Ok(_) => {
            info!("Calculation finished");
        }
        Err(e) => {
            error!("An error occurred: {}", e);
        }
    }
}
