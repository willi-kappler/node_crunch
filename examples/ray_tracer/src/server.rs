use log::{info, error, debug};
use image;

use node_crunch::{NCServer, NCJobStatus, NCConfiguration, NCError,
    Array2DChunk, ChunkList, ChunkData, NodeID, NCServerStarter};

use crate::{RayTracer1Opt, ServerData, NodeData};

/// This contains the final image and the list of chunks
#[derive(Debug, Clone)]
struct RayTracerServer {
    /// Width of the final image
    width: u64,
    /// Height of the final image
    height: u64,
    /// This holds the image data (pixels) for the final mandelbrot image.
    array2d_chunk: Array2DChunk<(u8, u8, u8)>,
    /// Book keeping chunk list, which node is processing which part of the image.
    chunk_list: ChunkList<ChunkData>,
}

impl RayTracerServer {
    /// Saves the image data to disk with a fancy color scheme.
    fn save_image(&self) {
        let (width, height) = self.array2d_chunk.dimensions();
        let mut buffer = image::ImageBuffer::new(width as u32, height as u32);

        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            let value = self.array2d_chunk.get(x as u64, y as u64);

            *pixel = image::Rgb([value.0, value.1, value.2]);
        }

        buffer.save("ray_tace1.png").unwrap();
    }

    /// Checks if the job (calculating the mandelbrot set) is already done.
    fn is_job_done(&self) -> bool {
        let (empty, processing, finished) = self.chunk_list.stats();
        debug!("Job status: empty: {}, processing: {}, finished: {}", empty, processing, finished);

        empty == 0 && processing == 0
    }
}

impl NCServer for RayTracerServer {
    type InitialDataT = ();
    type NewDataT = ServerData;
    type ProcessedDataT = NodeData;
    type CustomMessageT = ();

    /// Every node needs some data to process. Here this data is prepared for each node and some book keeping is saved in the chunks list.
    /// The whole ray tracing image is split up into equally sized pieces and processed separately.
    /// Returns the NCJobStatus that is checked by the server.
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus<Self::NewDataT>, NCError> {
        debug!("Server::prepare_data_for_node, node_id: {}", node_id);

        if let Some((i, free_chunk)) = self.chunk_list.get_next_free_chunk() {
            let data = &free_chunk.data;
            let data_for_node = ServerData {
                chunk_id: i as u64,
                x: data.x,
                y: data.y,
                width: data.width,
                height: data.height,
            };

            free_chunk.set_processing(node_id);
            // debug!("preparing chunk {} for node {}", i, node_id);
            // debug!("x: {}, y: {}, w: {}, h: {}", data_for_node.x, data_for_node.y, data_for_node.width, data_for_node.height);
            Ok(NCJobStatus::Unfinished(data_for_node))
        } else {
            if self.is_job_done() {
                Ok(NCJobStatus::Finished)
            } else {
                Ok(NCJobStatus::Waiting)
            }
        }
    }

    /// If one of the nodes has finished processing the small chunk the server writes the data back to the whole image Array2D.
    fn process_data_from_node(&mut self, node_id: NodeID, node_data: &Self::ProcessedDataT) -> Result<(), NCError> {
        debug!("Server::process_data_from_node, node_id: {}", node_id);

        let chunk_id = node_data.chunk_id;
        let source = &node_data.img;
        let current_chunk = &mut self.chunk_list.get(chunk_id as usize);
        // let chunk_data = &current_chunk.data;

        debug!("chunk_id: {}", chunk_id);
        // debug!("x: {}, y: {}, width: {}, height: {}", chunk_data.x, chunk_data.y, chunk_data.width, chunk_data.height);

        // let (cx, cy, cw, ch) = self.array2d_chunk.get_chunk_property(chunk_id);
        // debug!("cx: {}, cy: {}, cw: {}, ch: {}", cx, cy, cw, ch);

        if current_chunk.is_processing(node_id) {
            current_chunk.set_finished();
            self.array2d_chunk.set_chunk(chunk_id, &source)
        } else {
            error!("Mismatch data, should be Processing with node_id: {}, but is {:?}", node_id, current_chunk.node_id);
            Err(NCError::NodeIDMismatch(node_id, current_chunk.node_id))
        }
    }

    /// If some nodes have crashed or lost the network connection the internal chunks list is updated.
    fn heartbeat_timeout(&mut self, nodes: Vec<NodeID>) {
        self.chunk_list.heartbeat_timeout(&nodes)
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

    let array2d_chunk = Array2DChunk::new(options.width, options.height, options.chunk_size, options.chunk_size, (0, 0, 0));
    let mut chunk_list = ChunkList::new();
    chunk_list.initialize(&array2d_chunk);

    /*
    for i in 0..array2d_chunk.num_of_chunks() {
        let chunk = chunk_list.get(i as usize);
        let chunk_data = &chunk.data;
        debug!("i: {}, node_id: {}, x: {}, y: {}, w: {}, h: {}", i, chunk.node_id, chunk_data.x, chunk_data.y, chunk_data.width, chunk_data.height);
    }
    */

    let server = RayTracerServer {
        width: options.width,
        height: options.height,
        array2d_chunk, chunk_list,
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
