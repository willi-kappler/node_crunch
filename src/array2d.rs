//! This module contains helper structures to deal with 2D data.
//! Array2D and Array2DChunk take care of splitting up the 2D array into chunks
//! that can be sent to the node in order to process them.

use std::{error, fmt};

use serde::{Serialize, Deserialize};

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

    /// Returns the dimension for the whole Array2D data
    pub fn dimensions(&self) -> (u64, u64) {
        (self.array2d.width, self.array2d.height)
    }
}
