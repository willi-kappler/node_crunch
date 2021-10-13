use log::{info, error, debug};
use num::complex::Complex64;
use image;

use node_crunch::{NCServer, NCJobStatus, NCConfiguration, NCError,
    Array2DChunk, ChunkList, ChunkData, NodeID, NCServerStarter};

use crate::{Mandel1Opt, ServerData, NodeData};

/// This has the image data, mandelbrot set configuration and the chunks list.
#[derive(Debug, Clone)]
struct MandelServer {
    /// Start (upper left) of mandelbrot set in the complex plane.
    start: Complex64,
    /// Start (lower right) of mandelbrot set in the complex plane.
    end: Complex64,
    /// The step size for each pixel in x direction.
    x_step: f64,
    /// The step size for each pixel in y direction.
    y_step: f64,
    /// Maximum number of iteration as escape time limit.
    max_iter: u32,
    /// This holds the image data (pixels) for the final mandelbrot image.
    array2d_chunk: Array2DChunk<u32>,
    /// Book keeping chunk list, which node is processing which part of the image.
    chunk_list: ChunkList<ChunkData>,
}

impl MandelServer {
    /// Saves the image data to disk with a fancy color scheme.
    fn save_image(&self) {
        let (width, height) = self.array2d_chunk.dimensions();
        let mut buffer = image::ImageBuffer::new(width as u32, height as u32);

        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            let value = self.array2d_chunk.get(x as u64, y as u64);

            if (value & 1) == 0 {
                *pixel = image::Rgb([0 as u8, 0 as u8, 0 as u8]);
            } else {
                *pixel = image::Rgb([255 as u8, 255 as u8, 255 as u8]);
            }
        }

        buffer.save("mandel.png").unwrap();
    }

    /// Checks if the job (calculating the mandelbrot set) is already done.
    fn is_job_done(&self) -> bool {
        let (empty, processing, finished) = self.chunk_list.stats();
        debug!("Job status: empty: {}, processing: {}, finished: {}", empty, processing, finished);

        empty == 0 && processing == 0
    }
}

impl NCServer for MandelServer {
    type InitialDataT = ();
    type NewDataT = ServerData;
    type ProcessedDataT = NodeData;
    type CustomMessageT = ();

    /// Every node needs some data to process. Here this data is prepared for each node and some book keeping is saved in the chunks list.
    /// The whole mandelbrot image is split up into equally sized pieces and processed separately.
    /// Returns the NCJobStatus that is checked by the server.
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus<Self::NewDataT>, NCError> {
        debug!("Server::prepare_data_for_node, node_id: {}", node_id);

        if let Some((i, free_chunk)) = self.chunk_list.get_next_free_chunk() {
            let data_for_node = ServerData {
                chunk_id: i as u64,
                max_iter: self.max_iter,
                x: free_chunk.data.x,
                y: free_chunk.data.y,
                width: free_chunk.data.width,
                height: free_chunk.data.height,
                x_step: self.x_step,
                y_step: self.y_step,
                re: self.start.re,
                im: self.start.im,
            };

            free_chunk.set_processing(node_id);
            debug!("preparing chunk {} for node {}", i, node_id);
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
        let source = &node_data.source;
        let current_chunk = &mut self.chunk_list.get(chunk_id as usize);

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
pub fn run_server(options: Mandel1Opt) {
    let configuration = NCConfiguration {
        port: options.port,
        compress: true,
        encrypt: true,
        // The key should be read from a config file
        key: "ZKS1GQ3MYWEKFILSN6KESXU2GD9015CH".to_string(),
        ..Default::default()
    };

    let start = Complex64{re: -2.0, im: -1.5};
    let end = Complex64{re: 1.0, im: 1.5};
    let img_size = 10000;
    let max_iter = 10000;
    let x_step = (end.re - start.re) / (img_size as f64);
    let y_step = (end.im - start.im) / (img_size as f64);
    let chunk_size = 2000;
    let array2d_chunk = Array2DChunk::new(img_size, img_size, chunk_size, chunk_size, 0);
    let mut chunk_list = ChunkList::new();

    for i in 0..array2d_chunk.num_of_chunks() {
        let (x, y, width, height) = array2d_chunk.get_chunk_property(i);

        chunk_list.push(ChunkData { x, y, width, height });
    }

    let server = MandelServer {
        start, end, x_step, y_step, max_iter, array2d_chunk, chunk_list,
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
