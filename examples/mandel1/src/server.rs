use log::{info, error, debug};
use num::complex::Complex64;
use image;

use node_crunch::{NCServer, NCJobStatus, NCConfiguration, NCError,
    Array2DChunk, ChunkList, NodeID,
    nc_start_server, nc_decode_data, nc_encode_data};

use crate::{Mandel1Opt, ServerData, NodeData};

#[derive(Debug, Clone)]
struct ChunkData {
    x: u64,
    y: u64,
    width: u64,
    height: u64,
}

#[derive(Debug, Clone)]
struct MandelServer {
    start: Complex64,
    end: Complex64,
    x_step: f64,
    y_step: f64,
    max_iter: u32,
    array2d_chunk: Array2DChunk::<u32>,
    chunk_list: ChunkList<ChunkData>,
}

impl MandelServer {
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

    fn is_job_done(&self) -> bool {
        let (empty, processing, finished) = self.chunk_list.stats();
        debug!("Job status: empty: {}, processing: {}, finished: {}", empty, processing, finished);

        empty == 0 && processing == 0
    }
}

impl NCServer for MandelServer {
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus, NCError> {
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

            match nc_encode_data(&data_for_node) {
                Ok(data) => {
                    free_chunk.set_processing(node_id);
                    debug!("preparing chunk {} for node {}", i, node_id);
                    Ok(NCJobStatus::Unfinished(data))
                }
                Err(e) => {
                    error!("An error occurred while preparing the data for the Node: {}, error: {}", node_id, e);
                    Err(e)
                }
            }
        } else {
            if self.is_job_done() {
                Ok(NCJobStatus::Finished)
            } else {
                Ok(NCJobStatus::Waiting)
            }
        }
    }

    fn process_data_from_node(&mut self, node_id: NodeID, node_data: &[u8]) -> Result<(), NCError> {
        debug!("Server::process_data_from_node, node_id: {}", node_id);

        match nc_decode_data::<NodeData>(node_data) {
            Ok(node_data) => {
                let chunk_id = node_data.chunk_id;
                let source = node_data.source;
                let current_chunk = &mut self.chunk_list.get(chunk_id as usize);

                if current_chunk.is_processing(node_id) {
                    current_chunk.set_finished();
                    self.array2d_chunk.set_chunk(chunk_id, &source).map_err(|e| NCError::Array2D(e))
                } else {
                    error!("Missmatch data, should be Processing with node_id: {}, but is {:?}", node_id, current_chunk.node_id);
                    Err(NCError::NodeIDMismatch(node_id, current_chunk.node_id))
                }
            },
            Err(e) => {
                error!("An error occurred while processing the data from the Node: {}", e);
                Err(e)
            }
        }
    }

    fn heartbeat_timeout(&mut self, nodes: Vec<NodeID>) {
        self.chunk_list.heartbeat_timeout(&nodes)
    }

    fn finish_job(&mut self) {
        self.save_image();
    }
}

pub fn run_server(options: Mandel1Opt) {
    let configuration = NCConfiguration {
        port: options.port,
        ..Default::default()
    };

    let start = Complex64{re: -2.0, im: -1.5};
    let end = Complex64{re: 1.0, im: 1.5};
    let img_size = 10000;
    let max_iter = 10000;
    let x_step = (end.re - start.re) / (img_size as f64);
    let y_step = (end.im - start.im) / (img_size as f64);
    let chunk_size = 2000;
    let array2d_chunk = Array2DChunk::<u32>::new(img_size, img_size, chunk_size, chunk_size, 0);
    let mut chunk_list = ChunkList::new();

    for i in 0..array2d_chunk.num_of_chunks() {
        let (x, y, width, height) = array2d_chunk.get_chunk_property(i);

        chunk_list.push(ChunkData { x, y, width, height });
    }

    let server = MandelServer {
        start, end, x_step, y_step, max_iter, array2d_chunk, chunk_list,
    };

    match nc_start_server(server, configuration) {
        Ok(_) => {
            info!("Calculation finished");
        }
        Err(e) => {
            error!("An error occurred: {}", e);
        }
    }
}
