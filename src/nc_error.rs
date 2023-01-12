
#[derive(Debug)]
pub enum NCError {
    Array2DDimensionMismatch((u64, u64), (u64, u64)),
}
