use log::{info, error};

use node_crunch::{NCNode, NCError, NCConfiguration, Array2D, NCNodeStarter};

use ray_tracer::scene::Scene;
use ray_tracer::camera::perspective::PerspectiveCamera;
use ray_tracer::renderer::Renderer;

use crate::ray_scene::RayScene;

use crate::{RayTracer1Opt, ServerData, NodeData};

/// In this example the NCNode data struct has no useful data, just code.
struct RayTracerNode {
    width: u64,
    height: u64,
    scene: Scene<f64>,
    camera: PerspectiveCamera<f64>,
    gamma: f64,
}

impl NCNode for RayTracerNode {
    type InitialDataT = ();
    type NewDataT = ServerData;
    type ProcessedDataT = NodeData;
    type CustomMessageT = ();

    /// This processes the data that has been send from the server to this node.
    /// In here the whole number crunching is happening in this example the ray tracing image.
    /// The result is returned in a Ok(Self::ProcessedDataT).
    /// Return an error otherwise.
    fn process_data_from_server(&mut self, data: &Self::NewDataT) -> Result<Self::ProcessedDataT, NCError> {
        let mut array2d = Array2D::<(u8, u8, u8)>::new(data.width, data.height, (0, 0, 0));

        let renderer = Renderer::new(
            data.x as usize,
            (data.x + data.width) as usize,
            data.y as usize,
            (data.y + data.height) as usize,
            self.width as usize,
            self.height as usize,
            0, 2, false);
        let image = renderer.render(&self.scene, &self.camera);

        // TODO: calculate scene
        for x in 0..data.width {
            for y in 0..data.height {
                let index = (y * (image.width as u64) + x) as usize;
                let r = (image.data[3 * index].powf(1.0 / self.gamma) * 255.0) as u8;
                let g = (image.data[3 * index + 1].powf(1.0 / self.gamma) * 255.0) as u8;
                let b = (image.data[3 * index + 2].powf(1.0 / self.gamma) * 255.0) as u8;

                array2d.set(x, y, (r, g, b));
            }
        }

        let result = NodeData {
            chunk_id: data.chunk_id,
            img: array2d,
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

    let (scene, camera) = RayScene::load("scene1");

    let node = RayTracerNode{
        width: options.width,
        height: options.height,
        scene,
        camera,
        gamma: options.gamma,
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
