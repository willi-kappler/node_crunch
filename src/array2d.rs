use serde::{Serialize, Deserialize};

pub enum Array2DError {
    DimensionMismatch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array2D<T> {
    width: u64,
    height: u64,
    data: Vec<T>,
}

impl<T: Clone + Copy> Array2D<T> {
    pub fn new(width: u64, height: u64, initial: T) -> Array2D<T> {
        let data = vec![initial; (width * height) as usize];

        Array2D {
            width, height, data,
        }
    }

    fn index(&self, x: u64, y: u64) -> usize {
        ((self.width * y) + x) as usize
    }

    pub fn get(&self, x: u64, y: u64) -> T {
        self.data[self.index(x, y)]
    }

    pub fn set(&mut self, x: u64, y: u64, value: T) {
        let index = self.index(x, y);
        self.data[index] = value;
    }

    pub fn set_region(&mut self, dest_x: u64, dest_y: u64, source: &Array2D<T>) {
        for x in 0..source.width {
            for y in 0..source.height {
                self.set(dest_x + x, dest_y + y, source.get(x, y))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array2DChunk<T> {
    chunk_width: u64,
    chunk_height: u64,
    rest_width: u64, // If chunks do not fit in array2d this is the rest in x direction
    rest_height: u64, // If chunks do not fit in array2d this is the rest in y direction
    chunks_x: u64, // Number of chunks in x direction
    chunks_y: u64, // Number of chunks in y direction
    array2d: Array2D<T>, // Contains the 2d array with data
}

impl<T: Clone + Copy> Array2DChunk<T> {
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

    pub fn num_of_chunks(&self) -> u64 {
        self.chunks_x * self.chunks_y
    }

    pub fn get_chunk_property(&self, chunk_id: u64) -> (u64, u64, u64, u64) {
        let cy = chunk_id / self.chunks_x;
        let cx = chunk_id - (cy * self.chunks_x);

        let width = if (cx == (self.chunks_x - 1)) && (self.rest_width > 0) { self.rest_width } else { self.chunk_width };
        let height = if (cy == (self.chunks_y - 1)) && (self.rest_height > 0) { self.rest_height } else { self.chunk_height };

        (cx * self.chunk_width, cy * self.chunk_height, width, height)
    }

    pub fn set_chunk(&mut self, chunk_id: u64, source: &Array2D<T>) -> Result<(), Array2DError> {
        let (x, y, width, height) = self.get_chunk_property(chunk_id);

        if (width == source.width) && (height == source.height) {
            self.array2d.set_region(x, y, source);
            Ok(())
        } else {
            Err(Array2DError::DimensionMismatch)
        }
    }

    pub fn get(&self, x: u64, y: u64) -> T {
        self.array2d.get(x, y)
    }

    pub fn dimensions(&self) -> (u64, u64) {
        (self.array2d.width, self.array2d.height)
    }
}
