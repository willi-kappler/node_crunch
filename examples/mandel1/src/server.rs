use log::{info, error, debug};
use num::{complex::Complex64};
use image;

use node_crunch::{NCServer, NCJobStatus, NCConfiguration, NCError, Array2DChunk,
    nc_start_server, nc_decode_data, nc_encode_data};

use crate::{Mandel1Opt, ServerData, NodeData};

#[derive(Debug, Clone, PartialEq)]
enum ChunkStatus {
    Empty,
    Processing,
    Finished,
}

#[derive(Debug, Clone)]
struct Chunk {
    x: u64,
    y: u64,
    width: u64,
    height: u64,
    node_id: u64,
    status: ChunkStatus,
}

impl Chunk {
    fn set_empty(&mut self) {
        self.status = ChunkStatus::Empty
    }

    fn is_empty(&self) -> bool {
        self.status == ChunkStatus::Empty
    }

    fn set_processing(&mut self, node_id: u64) {
        self.status = ChunkStatus::Processing;
        self.node_id = node_id;
    }

    fn is_processing(&self, node_id: u64) -> bool {
        self.status == ChunkStatus::Processing &&
        self.node_id == node_id
    }

    fn set_finished(&mut self) {
        self.status = ChunkStatus::Finished;
    }
}

#[derive(Debug, Clone)]
struct MandelServer {
    start: Complex64,
    end: Complex64,
    x_step: f64,
    y_step: f64,
    max_iter: u32,
    array2d_chunk: Array2DChunk::<u32>,
    all_chunks: Vec<Chunk>,
}

impl MandelServer {
    fn save_image(&self) {
        let (width, height) = self.array2d_chunk.dimensions();
        let mut buffer = image::ImageBuffer::new(width as u32, height as u32);

        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            let value = self.array2d_chunk.get(x as u64, y as u64);
            if (0 >= value) && (value < 256) {
                *pixel = image::Rgb([value as u8, 0, 0]);
            } else if (256 >= value) && (value < 512) {
                *pixel = image::Rgb([255, (value - 256) as u8, 0]);
            } else if (512 >= value) && (value < 768) {
                *pixel = image::Rgb([(767 - value) as u8, 255, 0]);
            } else {
                *pixel = image::Rgb([0, 255, (value - 768) as u8]);
            }
        }

        buffer.save("mandel.png").unwrap();
    }
}

impl NCServer for MandelServer {
    fn prepare_data_for_node(&mut self, node_id: u64) -> Result<Option<Vec<u8>>, NCError> {
        debug!("Server::prepare_data_for_node, node_id: {}", node_id);

        for i in 0..self.all_chunks.len() {
            let current_chunk = &mut self.all_chunks[i];

            if current_chunk.is_empty() {
                let data_for_node = ServerData {
                    chunk_id: i as u64,
                    max_iter: self.max_iter,
                    x: current_chunk.x,
                    y: current_chunk.y,
                    width: current_chunk.width,
                    height: current_chunk.height,
                    x_step: self.x_step,
                    y_step: self.y_step,
                    re: self.start.re,
                    im: self.start.im,
                };

                match nc_encode_data(&data_for_node) {
                    Ok(data) => {
                        current_chunk.set_processing(node_id);
                        debug!("preparing chunk {} for node {}", i, node_id);
                        return Ok(Some(data))
                    }
                    Err(e) => {
                        error!("An error occurred while preparing the data for the Node: {}, error: {}", node_id, e);
                        return Err(e)
                    },
                }
            }
        }

        return Ok(None)
    }

    fn process_data_from_node(&mut self, node_id: u64, node_data: &Vec<u8>) -> Result<(), NCError> {
        debug!("Server::process_data_from_node, node_id: {}", node_id);

        match nc_decode_data::<NodeData>(node_data) {
            Ok(node_data) => {
                let chunk_id = node_data.chunk_id;
                let source = node_data.source;
                let current_chunk = &mut self.all_chunks[chunk_id as usize];

                if current_chunk.is_processing(node_id) {
                    current_chunk.set_finished();
                    self.array2d_chunk.set_chunk(chunk_id, &source).map_err(|_| NCError::Custom(2))
                } else {
                    error!("Missmatch data, should be Processing with node_id: {}, but is {:?}", node_id, current_chunk.node_id);
                    Err(NCError::Custom(1))
                }
            },
            Err(e) => {
                error!("An error occurred while processing the data from the Node: {}", e);
                Err(e)
            }
        }
    }

    fn job_status(&self) -> NCJobStatus {
        let mut empty = 0;
        let mut processing = 0;
        let mut finished = 0;

        for chunk in self.all_chunks.iter() {
            match chunk.status {
                ChunkStatus::Empty => empty = empty + 1,
                ChunkStatus::Processing => processing = processing + 1,
                ChunkStatus::Finished => finished = finished + 1,
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

    fn heartbeat_timeout(&mut self, node_id: u64) {
        info!("Heartbeat timeout, node_id: {}", node_id);

        for chunk in self.all_chunks.iter_mut() {
            if chunk.is_processing(node_id) {
                chunk.set_empty()
            }
        }
    }
}

pub fn run_server(options: Mandel1Opt) {
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
    let chunk_size = 256;
    let array2d_chunk = Array2DChunk::<u32>::new(img_size, img_size, chunk_size, chunk_size, 0);
    let mut all_chunks = Vec::new();

    for i in 0..array2d_chunk.num_of_chunks() {
        let (x, y, width, height) = array2d_chunk.get_chunk_property(i);
        let chunk = Chunk {
            x, y, width, height, node_id: 0, status: ChunkStatus::Empty
        };

        all_chunks.push(chunk);
    }

    let server = MandelServer {
        start, end, x_step, y_step,
        max_iter, array2d_chunk, all_chunks,
    };

    match nc_start_server(server, configuration) {
        Ok(server) => {
            info!("Calculation finished");
            server.save_image();
        }
        Err(e) => {
            error!("An error occurred: {}", e);
        }
    }
}
