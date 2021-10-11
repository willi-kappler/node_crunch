use log::{info, error, debug};
use image::{RgbImage, Rgb};

use node_crunch::{NCServer, NCJobStatus, NCConfiguration, NCError,
    NodeID, NCServerStarter};

use crate::{RayTracer1Opt, ServerData, NodeData};


/// This contains teh final image and the list of chunks
#[derive(Debug, Clone)]
struct RayTracerServer {
    width: usize,
    height: usize,
}

impl RayTracerServer {
    /// Saves the image data to disk with a fancy color scheme.
    fn save_image(&self) {
        let mut buffer = RgbImage::new(self.width as u32, self.height as u32);

        // TODO: process image data

        buffer.save("ray_tace1.png").unwrap();
    }

    /// Checks if the job (calculating the mandelbrot set) is already done.
    fn is_job_done(&self) -> bool {
        // TODO: determine when the job is done

        false
    }
}

impl NCServer for RayTracerServer {
    type InitialDataT = ();
    type NewDataT = ServerData;
    type ProcessedDataT = NodeData;
    type CustomMessageT = ();

    /// Every node needs some data to process. Here this data is prepared for each node and some book keeping is saved in the chunks list.
    /// The whole mandelbrot image is split up into equally sized pieces and processed separately.
    /// Returns the NCJobStatus that is checked by the server.
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus<Self::NewDataT>, NCError> {
        debug!("Server::prepare_data_for_node, node_id: {}", node_id);

        // TODO: prepare data for node

        if self.is_job_done() {
            Ok(NCJobStatus::Finished)
        } else {
            Ok(NCJobStatus::Waiting)
        }

    }

    /// If one of the nodes has finished processing the small chunk the server writes the data back to the whole image Array2D.
    fn process_data_from_node(&mut self, node_id: NodeID, node_data: &Self::ProcessedDataT) -> Result<(), NCError> {
        debug!("Server::process_data_from_node, node_id: {}", node_id);

        Ok(())
    }

    /// If some nodes have crashed or lost the network connection the internal chunks list is updated.
    fn heartbeat_timeout(&mut self, nodes: Vec<NodeID>) {
        // TODO: handle broken node
    }

    /// When all processing is done the server calls this method. Here we just save the final image to disk.
    fn finish_job(&mut self) {
        self.save_image();
    }
}

/// Starts the server with the given configuration
pub fn run_server(options: RayTracer1Opt) {
    let configuration = NCConfiguration {
        port: options.port,
        compress: true,
        encrypt: true,
        // The key should be read from a config file
        key: "ZKS1GQ3MYWEKFILSN6KESXU2GD9015CH".to_string(),
        ..Default::default()
    };

    let server = RayTracerServer {
        width: 1024,
        height: 768,
    };

    let mut server_starter = NCServerStarter::new(configuration);

    match server_starter.start(server) {
        Ok(_) => {
            info!("Calculation finished");
        }
        Err(e) => {
            error!("An error occurred: {}", e);
        }
    }
}
