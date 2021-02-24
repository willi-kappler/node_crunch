//! This module contains helper structures to deal with 2D data.
//! Array2D and Array2DChunk take care of splitting up the 2D array into chunks
//! that can be sent to the node in order to process them.

use std::{error, fmt};

use serde::{Serialize, Deserialize};

use crate::nc_node_info::NodeID;

/// Currently only one error when the dimension do not match.
#[derive(Debug)]
pub enum Array2DError {
    /// The given dimension does not match with the dimension of the Array2DChunk.
    DimensionMismatch,
}

impl fmt::Display for Array2DError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Array2DError::DimensionMismatch => write!(f, "Dimensions do not match: "),
        }
    }
}

impl error::Error for Array2DError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Array2DError::DimensionMismatch => None,
        }
    }
}

/// Contains the 2D data, the width and the height od the 2D array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array2D<T> {
    /// The width of the 2D array.
    width: u64,
    /// The height of the 2D array.
    height: u64,
    /// The actual data.
    data: Vec<T>,
}

impl<T: Clone + Copy> Array2D<T> {
    /// Creates a new Array2D with the given dimension and the initial fill value.
    pub fn new(width: u64, height: u64, initial: T) -> Array2D<T> {
        let data = vec![initial; (width * height) as usize];

        Array2D {
            width, height, data,
        }
    }

    /// Calculates the correct index into the Vector for the given (x, y) position.
    fn index(&self, x: u64, y: u64) -> usize {
        ((self.width * y) + x) as usize
    }

    /// Returns the value at the given (x, y) position.
    pub fn get(&self, x: u64, y: u64) -> T {
        self.data[self.index(x, y)]
    }

    /// Sets the value at the given (x, y) position.
    pub fn set(&mut self, x: u64, y: u64, value: T) {
        let index = self.index(x, y);
        self.data[index] = value;
    }

    // Sets a whole 2D region of data values.
    pub fn set_region(&mut self, dest_x: u64, dest_y: u64, source: &Array2D<T>) {
        for x in 0..source.width {
            for y in 0..source.height {
                self.set(dest_x + x, dest_y + y, source.get(x, y))
            }
        }
    }
}

// A data structure that can be split up into several chunks that are send to individual nodes to be processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array2DChunk<T> {
    /// The width of each individual chunk.
    chunk_width: u64,
    /// The height of each individual chunk.
    chunk_height: u64,
    /// If chunks do not fit in array2d this is the rest in x direction.
    rest_width: u64,
    /// If chunks do not fit in array2d this is the rest in y direction.
    rest_height: u64,
    /// Number of chunks in x direction.
    chunks_x: u64,
    /// Number of chunks in y direction.
    chunks_y: u64,
    /// Contains the 2D array with data.
    array2d: Array2D<T>,
}

impl<T: Clone + Copy> Array2DChunk<T> {
    /// Creates a new Array2DChunk with the given width, height, chunk_width and chunk_height.
    pub fn new(width: u64, height: u64, chunk_width: u64, chunk_height: u64, initial: T) -> Array2DChunk<T> {
        let a = width / chunk_width;
        let rest_width = width - (a * chunk_width);

        let b = height / chunk_height;
        let rest_height = height - (b * chunk_height);

        let chunks_x = if rest_width == 0 {a} else {a + 1};
        let chunks_y = if rest_height == 0 {b} else {b + 1};

        let array2d = Array2D::new(width, height, initial);

        Array2DChunk {
            chunk_width, chunk_height,
            rest_width, rest_height,
            chunks_x, chunks_y, array2d,
        }
    }

    /// Returns the total number of chunks.
    pub fn num_of_chunks(&self) -> u64 {
        self.chunks_x * self.chunks_y
    }

    /// Returns the property of the chunk for the given chunk ID.
    /// The (x, y) and the width and the height of the chunk.
    pub fn get_chunk_property(&self, chunk_id: u64) -> (u64, u64, u64, u64) {
        let cy = chunk_id / self.chunks_x;
        let cx = chunk_id - (cy * self.chunks_x);

        let width = if (cx == (self.chunks_x - 1)) && (self.rest_width > 0) { self.rest_width } else { self.chunk_width };
        let height = if (cy == (self.chunks_y - 1)) && (self.rest_height > 0) { self.rest_height } else { self.chunk_height };

        (cx * self.chunk_width, cy * self.chunk_height, width, height)
    }

    /// Sets the chunk data for the given chunk id.
    pub fn set_chunk(&mut self, chunk_id: u64, source: &Array2D<T>) -> Result<(), Array2DError> {
        let (x, y, width, height) = self.get_chunk_property(chunk_id);

        if (width == source.width) && (height == source.height) {
            self.array2d.set_region(x, y, source);
            Ok(())
        } else {
            Err(Array2DError::DimensionMismatch)
        }
    }

    /// Returns the data at the given (x, y) position
    pub fn get(&self, x: u64, y: u64) -> T {
        self.array2d.get(x, y)
    }

    /// Returns the dimension (width, height) for the whole Array2D data
    pub fn dimensions(&self) -> (u64, u64) {
        (self.array2d.width, self.array2d.height)
    }
}

/// A Chunk of data can have three states.
#[derive(Debug, Clone, PartialEq)]
enum ChunkStatus {
    /// 1. empty: no node has been assigned to this data.
    Empty,
    /// 2. processing: at least one node is processing the data.
    Processing,
    /// 3. finished: this piece of data has been processed sucessfully.
    Finished,
}

/// The actual data and some book keeping information.
#[derive(Debug, Clone)]
pub struct Chunk<T> {
    /// The data itself.
    pub data: T,
    /// ID of the node that is currently processing the data.
    pub node_id: NodeID,
    /// Has the data already been processed ? Is is not assigned yet ?
    status: ChunkStatus,
}

impl<T> Chunk<T> {
    /// Sets the chunk status to empty.
    pub fn set_empty(&mut self) {
        self.status = ChunkStatus::Empty
    }

    /// Checks if the chunk has been assigned yet or not.
    pub fn is_empty(&self) -> bool {
        self.status == ChunkStatus::Empty
    }

    /// Sets the chunk status to processing with the corresponding node id.
    pub fn set_processing(&mut self, node_id: NodeID) {
        self.status = ChunkStatus::Processing;
        self.node_id = node_id;
    }

    /// Checks if the chunk is currently beeing processed.
    pub fn is_processing(&self, node_id: NodeID) -> bool {
        self.status == ChunkStatus::Processing &&
        self.node_id == node_id
    }

    /// Checks if the chunk is finished.
    pub fn set_finished(&mut self) {
        self.status = ChunkStatus::Finished;
    }
}

/// A list of chunks and some helper functions.
#[derive(Debug, Clone)]
pub struct ChunkList<T> {
    chunks: Vec<Chunk<T>>
}

impl<T> ChunkList<T> {
    /// Creates a new and empty chunk list.
    pub fn new() -> Self {
        ChunkList{ chunks: Vec::new() }
    }

    /// Returns some statistics about the chunks in the list as a three tuple:
    /// 1. empty: how many chunks have not been assigned yet
    /// 2. processing: how many chunks are assigned to nodes.
    /// 3. finished: how many chunks are done with processing.
    /// (empty, processing, finished)
    pub fn stats(&self) -> (u64, u64, u64) {
        let mut empty: u64 = 0;
        let mut processing: u64 = 0;
        let mut finished: u64 = 0;

        for chunk in self.chunks.iter() {
            match chunk.status {
                ChunkStatus::Empty => empty += 1,
                ChunkStatus::Processing => processing += 1,
                ChunkStatus::Finished => finished += 1,
            }
        }

        (empty, processing, finished)
    }

    /// If there is a free chunk in the list return the index and a mutable reference to it.
    /// Else return None if all chunks are in processing or finished state.
    pub fn get_next_free_chunk(&mut self) -> Option<(usize, &mut Chunk<T>)> {
        self.chunks.iter().position(|chunk| chunk.is_empty()).map(move |index| (index, &mut self.chunks[index]))
    }

    /// Returns a mutable reference to the chunk at the given index.
    pub fn get(&mut self, index: usize) -> &mut Chunk<T> {
        &mut self.chunks[index]
    }

    /// Some nodes may have crashed or lost the network connection.
    /// Sets all the chunks that these nodes have been processing to the empty state ChunkStatus::Empty.
    pub fn heartbeat_timeout(&mut self, nodes: &[NodeID]) {
        for chunk in self.chunks.iter_mut() {
            for node_id in nodes.iter() {
                if chunk.is_processing(*node_id) {
                    chunk.set_empty()
                }
            }
        }
    }

    /// Adds a new chunk with the given data to the list of chunks.
    pub fn push(&mut self, data: T) {
        self.chunks.push(Chunk{ data, node_id: NodeID::random(), status: ChunkStatus::Empty});
    }
}
