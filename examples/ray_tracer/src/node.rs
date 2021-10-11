use log::{info, error};

use node_crunch::{NCNode, NCError, NCConfiguration, NCNodeStarter};

use crate::{RayTracer1Opt, ServerData, NodeData};

/// In this example the NCNode data struct has no useful data, just code.
struct RayTracerNode {
    width: usize,
    height: usize,
}

impl NCNode for RayTracerNode {
    type InitialDataT = ();
    type NewDataT = ServerData;
    type ProcessedDataT = NodeData;
    type CustomMessageT = ();

    /// This processes the data that has been send from the server to this node.
    /// In here the whole number crunching is happening in this example the mandelbrot set.
    /// The result is returned in a Ok(Self::ProcessedDataT).
    /// Return an error otherwise.
    fn process_data_from_server(&mut self, data: &Self::NewDataT) -> Result<Self::ProcessedDataT, NCError> {
        // TODO: calculate scene
        let result = NodeData {
            chunk_id: 0,
            img: true,
        };

        Ok(result)
    }
}

/// Starts the node with the given configuration.
pub fn run_node(options: RayTracer1Opt) {
    let configuration = NCConfiguration {
        port: options.port,
        address: options.ip,
        compress: true,
        encrypt: true,
        // The key should be read from a config file
        key: "ZKS1GQ3MYWEKFILSN6KESXU2GD9015CH".to_string(),
        ..Default::default()
    };

    let node = RayTracerNode{
        width: 1024,
        height: 768,
    };
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
