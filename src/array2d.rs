//! This module contains helper structures to deal with 2D data.
//! [`Array2D`] and [`Array2DChunk`] take care of splitting up the 2D array into chunks
//! that can be sent to the node in order to process them.

use std::slice::{Chunks, ChunksMut};

use serde::{Serialize, Deserialize};

use crate::nc_node_info::NCNodeInfo;
use crate::nc_error::NCError;

/// Contains the 2D data, the width and the height of the 2D array.
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
    /// Creates a new [`Array2D`] with the given dimension and the initial fill value.
    pub fn new(width: u64, height: u64, initial: T) -> Array2D<T> {
        assert!(width > 0);
        assert!(height > 0);

        let data = vec![initial; (width * height) as usize];

        Array2D {
            width, height, data,
        }
    }

    /// Calculates the correct index into the [`Vec`] for the given `(x, y)` position.
    fn index(&self, x: u64, y: u64) -> usize {
        ((self.width * y) + x) as usize
    }

    /// Returns the value at the given `(x, y)` position.
    pub fn get(&self, x: u64, y: u64) -> T {
        self.data[self.index(x, y)]
    }

    /// Sets the value at the given `(x, y)` position.
    pub fn set(&mut self, x: u64, y: u64, value: T) {
        let index = self.index(x, y);
        self.data[index] = value;
    }

    /// Sets a whole 2D region of data values.
    pub fn set_region(&mut self, dest_x: u64, dest_y: u64, source: &Array2D<T>) {
        for x in 0..source.width {
            for y in 0..source.height {
                self.set(dest_x + x, dest_y + y, source.get(x, y))
            }
        }
    }

    /// Returns an iterator over all rows.
    /// This can be used with rayons `par_bridge()` for example.
    pub fn split_rows(&self) -> Chunks<'_, T> {
        self.data.chunks(self.width as usize)
    }

    /// Returns a mutable iterator over all rows.
    /// This can be used with rayons `par_bridge()` for example.
    pub fn split_row_mut(&mut self) -> ChunksMut<'_, T> {
        self.data.chunks_mut(self.width as usize)
    }
}

/// A data structure that can be split up into several chunks that are send to individual nodes to be processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array2DChunk<T> {
    /// The width of each individual chunk.
    chunk_width: u64,
    /// The height of each individual chunk.
    chunk_height: u64,
    /// If chunks do not fit in `array2d` this is the rest in x direction.
    rest_width: u64,
    /// If chunks do not fit in `array2d` this is the rest in y direction.
    rest_height: u64,
    /// Number of chunks in x direction.
    chunks_x: u64,
    /// Number of chunks in y direction.
    chunks_y: u64,
    /// Contains the 2D array with data.
    array2d: Array2D<T>,
}

impl<T: Clone + Copy> Array2DChunk<T> {
    /// Creates a new [`Array2DChunk`] with the given `width`, `height`, `chunk_width` and `chunk_height`.
    pub fn new(width: u64, height: u64, chunk_width: u64, chunk_height: u64, initial: T) -> Array2DChunk<T> {
        assert!(width > 0);
        assert!(height > 0);
        assert!(chunk_width > 0);
        assert!(chunk_height > 0);
        assert!(chunk_width < width);
        assert!(chunk_height < height);

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
    /// The `(x, y)` and the width and the height of the chunk.
    pub fn get_chunk_property(&self, chunk_id: u64) -> (u64, u64, u64, u64) {
        let cy = chunk_id / self.chunks_x;
        let cx = chunk_id - (cy * self.chunks_x);

        let width = if (cx == (self.chunks_x - 1)) && (self.rest_width > 0) { self.rest_width } else { self.chunk_width };
        let height = if (cy == (self.chunks_y - 1)) && (self.rest_height > 0) { self.rest_height } else { self.chunk_height };

        (cx * self.chunk_width, cy * self.chunk_height, width, height)
    }

    /// Sets the chunk data for the given chunk id.
    pub fn set_chunk(&mut self, chunk_id: u64, source: &Array2D<T>) -> Result<(), NCError> {
        let (x, y, width, height) = self.get_chunk_property(chunk_id);

        if (width == source.width) && (height == source.height) {
            self.array2d.set_region(x, y, source);
            Ok(())
        } else {
            Err(NCError::Array2DDimensionMismatch((width, height), (source.width, source.height)))
        }
    }

    /// Returns the data at the given `(x, y)` position
    pub fn get(&self, x: u64, y: u64) -> T {
        self.array2d.get(x, y)
    }

    /// Returns the dimension `(width, height)` for the whole [`Array2D`] data
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
    /// 3. finished: this piece of data has been processed successfully.
    Finished,
}

/// The actual data and some book keeping information.
#[derive(Debug, Clone)]
pub struct Chunk<T> {
    /// The data itself.
    pub data: T,
    /// ID of the node that is currently processing the data.
    pub node_info: NCNodeInfo,
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
    pub fn set_processing(&mut self, node_info: NCNodeInfo) {
        self.status = ChunkStatus::Processing;
        self.node_info = node_info;
    }

    /// Checks if the chunk is currently being processed.
    pub fn is_processing(&self, node_info: &NCNodeInfo) -> bool {
        self.status == ChunkStatus::Processing &&
        self.node_info == *node_info
    }

    /// Checks if the chunk is finished.
    pub fn set_finished(&mut self) {
        self.status = ChunkStatus::Finished;
    }
}

/// A list of chunks and some helper methods.
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
    /// 1. `empty`: how many chunks have not been assigned yet
    /// 2. `processing`: how many chunks are assigned to nodes.
    /// 3. `finished`: how many chunks are done with processing.
    /// `(empty, processing, finished)`
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
    /// Else return [`None`] if all chunks are in processing or finished state.
    pub fn get_next_free_chunk(&mut self) -> Option<(usize, &mut Chunk<T>)> {
        self.chunks.iter().position(|chunk| chunk.is_empty()).map(move |index| (index, &mut self.chunks[index]))
    }

    /// Returns a mutable reference to the chunk at the given index.
    pub fn get(&mut self, index: usize) -> &mut Chunk<T> {
        &mut self.chunks[index]
    }

    /// Some nodes may have crashed or lost the network connection.
    /// Sets all the chunks that these nodes have been processing to the empty state.
    pub fn heartbeat_timeout(&mut self, nodes: &[NCNodeInfo]) {
        for chunk in self.chunks.iter_mut() {
            for node_id in nodes.iter() {
                if chunk.is_processing(node_id) {
                    chunk.set_empty()
                }
            }
        }
    }

    /// Adds a new chunk with the given data to the list of chunks.
    pub fn push(&mut self, data: T) {
        self.chunks.push(Chunk{ data, node_info: NCNodeInfo::empty(), status: ChunkStatus::Empty});
    }
}

impl ChunkList<ChunkData> {
    pub fn initialize<T: Clone + Copy>(&mut self, array2d_chunk: &Array2DChunk<T>) {
        for i in 0..array2d_chunk.num_of_chunks() {
            let (x, y, width, height) = array2d_chunk.get_chunk_property(i);

            self.push(ChunkData { x, y, width, height });
        }
    }
}

/// This is the data that is stored in the chunks list.
#[derive(Debug, Clone)]
pub struct ChunkData {
    /// X position of the chunk inside the Array2D = x position in the final image.
    pub x: u64,
    /// A position of the chunk inside the Array2D = y position in the final image.
    pub y: u64,
    /// width of the chunk inside the Array2D.
    pub width: u64,
    /// height of the chunk inside the Array2D.
    pub height: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_a2d_new1() {
        let _a2d = Array2D::new(0, 1, 99);
    }

    #[test]
    #[should_panic]
    fn test_a2d_new2() {
        let _a2d = Array2D::new(1, 0, 99);
    }

    #[test]
    fn test_a2d_index() {
        let a2d = Array2D::new(10, 10, 0);

        assert_eq!(a2d.index(0, 0), 0);
        assert_eq!(a2d.index(1, 0), 1);
        assert_eq!(a2d.index(0, 1), 10);
        assert_eq!(a2d.index(1, 1), 11);
        assert_eq!(a2d.index(9, 9), 99);
    }

    #[test]
    fn test_a2d_set_and_get() {
        let mut a2d = Array2D::new(10, 10, 0);

        assert_eq!(a2d.get(0, 0), 0);
        a2d.set(0, 0, 55);
        assert_eq!(a2d.get(0, 0), 55);

        assert_eq!(a2d.get(4, 8), 0);
        a2d.set(4, 8, 123);
        assert_eq!(a2d.get(4, 8), 123);

        assert_eq!(a2d.get(9, 9), 0);
        a2d.set(9, 9, 999);
        assert_eq!(a2d.get(9, 9), 999);
    }

    #[test]
    fn test_a2d_set_region1() {
        let mut a2d = Array2D::new(10, 10, 0);

        let region = Array2D::new(1, 1, 789);

        assert_eq!(a2d.get(6, 5), 0);
        a2d.set_region(6, 5, &region);
        assert_eq!(a2d.get(6, 5), 789);
    }

    #[test]
    fn test_a2d_set_region2() {
        let mut a2d = Array2D::new(10, 10, 0);

        let region = Array2D::new(2, 2, 2345);

        assert_eq!(a2d.get(2, 7), 0);
        assert_eq!(a2d.get(3, 7), 0);
        assert_eq!(a2d.get(2, 8), 0);
        assert_eq!(a2d.get(3, 8), 0);
        a2d.set_region(2, 7, &region);
        assert_eq!(a2d.get(2, 7), 2345);
        assert_eq!(a2d.get(3, 7), 2345);
        assert_eq!(a2d.get(2, 8), 2345);
        assert_eq!(a2d.get(3, 8), 2345);
    }

    #[test]
    fn test_a2d_set_region3() {
        let mut a2d = Array2D::new(2, 2, 0);

        let region = Array2D::new(2, 2, 8765);

        assert_eq!(a2d.get(0, 0), 0);
        assert_eq!(a2d.get(1, 0), 0);
        assert_eq!(a2d.get(0, 1), 0);
        assert_eq!(a2d.get(1, 1), 0);
        a2d.set_region(0, 0, &region);
        assert_eq!(a2d.get(0, 0), 8765);
        assert_eq!(a2d.get(1, 0), 8765);
        assert_eq!(a2d.get(0, 1), 8765);
        assert_eq!(a2d.get(1, 1), 8765);
    }

    #[test]
    fn test_a2d_chunk_new1() {
        let a2d_chunk = Array2DChunk::new(100, 100, 20, 20, 0);

        assert_eq!(a2d_chunk.chunk_width, 20);
        assert_eq!(a2d_chunk.chunk_height, 20);
        assert_eq!(a2d_chunk.rest_width, 0);
        assert_eq!(a2d_chunk.rest_height, 0);
        assert_eq!(a2d_chunk.chunks_x, 5);
        assert_eq!(a2d_chunk.chunks_y, 5);
    }

    #[test]
    fn test_a2d_chunk_new2() {
        let a2d_chunk = Array2DChunk::new(100, 100, 21, 21, 0);

        assert_eq!(a2d_chunk.chunk_width, 21);
        assert_eq!(a2d_chunk.chunk_height, 21);
        assert_eq!(a2d_chunk.rest_width, 16);
        assert_eq!(a2d_chunk.rest_height, 16);
        assert_eq!(a2d_chunk.chunks_x, 5);
        assert_eq!(a2d_chunk.chunks_y, 5);
    }

    #[test]
    #[should_panic]
    fn test_a2d_chunk_new3() {
        let _a2d_chunk = Array2DChunk::new(0, 100, 10, 10, 0);
    }

    #[test]
    #[should_panic]
    fn test_a2d_chunk_new4() {
        let _a2d_chunk = Array2DChunk::new(100, 0, 10, 10, 0);
    }

    #[test]
    #[should_panic]
    fn test_a2d_chunk_new5() {
        let _a2d_chunk = Array2DChunk::new(100, 100, 0, 10, 0);
    }

    #[test]
    #[should_panic]
    fn test_a2d_chunk_new6() {
        let _a2d_chunk = Array2DChunk::new(100, 100, 10, 0, 0);
    }

    #[test]
    #[should_panic]
    fn test_a2d_chunk_new7() {
        let _a2d_chunk = Array2DChunk::new(100, 100, 110, 10, 0);
    }

    #[test]
    #[should_panic]
    fn test_a2d_chunk_new8() {
        let _a2d_chunk = Array2DChunk::new(100, 100, 10, 110, 0);
    }

    #[test]
    fn test_a2d_chunk_num_of_chunks1() {
        let a2d_chunk = Array2DChunk::new(100, 100, 20, 20, 0);

        assert_eq!(a2d_chunk.num_of_chunks(), 25);
    }

    #[test]
    fn test_a2d_chunk_num_of_chunks2() {
        let a2d_chunk = Array2DChunk::new(100, 100, 21, 21, 0);

        assert_eq!(a2d_chunk.num_of_chunks(), 25);
    }

    #[test]
    fn test_a2d_chunk_get_chunk_property1() {
        let a2d_chunk = Array2DChunk::new(120, 120, 40, 40, 0);

        assert_eq!(a2d_chunk.num_of_chunks(), 9);

        assert_eq!(a2d_chunk.get_chunk_property(0), (0, 0, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(1), (40, 0, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(2), (80, 0, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(3), (0, 40, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(4), (40, 40, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(5), (80, 40, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(6), (0, 80, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(7), (40, 80, 40, 40));
        assert_eq!(a2d_chunk.get_chunk_property(8), (80, 80, 40, 40));
    }

    #[test]
    fn test_a2d_chunk_get_chunk_property2() {
        let a2d_chunk = Array2DChunk::new(120, 120, 41, 41, 0);

        assert_eq!(a2d_chunk.num_of_chunks(), 9);

        assert_eq!(a2d_chunk.get_chunk_property(0), (0, 0, 41, 41));
        assert_eq!(a2d_chunk.get_chunk_property(1), (41, 0, 41, 41));
        assert_eq!(a2d_chunk.get_chunk_property(2), (82, 0, 38, 41));
        assert_eq!(a2d_chunk.get_chunk_property(3), (0, 41, 41, 41));
        assert_eq!(a2d_chunk.get_chunk_property(4), (41, 41, 41, 41));
        assert_eq!(a2d_chunk.get_chunk_property(5), (82, 41, 38, 41));
        assert_eq!(a2d_chunk.get_chunk_property(6), (0, 82, 41, 38));
        assert_eq!(a2d_chunk.get_chunk_property(7), (41, 82, 41, 38));
        assert_eq!(a2d_chunk.get_chunk_property(8), (82, 82, 38, 38));
    }

    #[test]
    fn test_a2d_chunk_set_chunk1() {
        let mut a2d_chunk = Array2DChunk::new(6, 6, 2, 2, 0);
        let a2d = Array2D::new(2, 2, 112233);

        assert_eq!(a2d_chunk.get(2, 0), 0);
        assert_eq!(a2d_chunk.get(2, 1), 0);
        assert_eq!(a2d_chunk.get(3, 0), 0);
        assert_eq!(a2d_chunk.get(3, 1), 0);
        a2d_chunk.set_chunk(1, &a2d).unwrap();
        assert_eq!(a2d_chunk.get(2, 0), 112233);
        assert_eq!(a2d_chunk.get(2, 1), 112233);
        assert_eq!(a2d_chunk.get(3, 0), 112233);
        assert_eq!(a2d_chunk.get(3, 1), 112233);
    }

    #[test]
    fn test_a2d_chunk_set_chunk2() {
        let mut a2d_chunk = Array2DChunk::new(6, 6, 2, 2, 0);
        let a2d = Array2D::new(2, 2, 456456);

        assert_eq!(a2d_chunk.get(2, 2), 0);
        assert_eq!(a2d_chunk.get(2, 3), 0);
        assert_eq!(a2d_chunk.get(3, 2), 0);
        assert_eq!(a2d_chunk.get(3, 3), 0);
        a2d_chunk.set_chunk(4, &a2d).unwrap();
        assert_eq!(a2d_chunk.get(2, 2), 456456);
        assert_eq!(a2d_chunk.get(2, 3), 456456);
        assert_eq!(a2d_chunk.get(3, 2), 456456);
        assert_eq!(a2d_chunk.get(3, 3), 456456);
    }
}
