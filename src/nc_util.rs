use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};

use crate::nc_error::{NCError};

pub fn nc_encode_data<T: Serialize>(data: &T) -> Result<Vec<u8>, NCError> {
    serialize(data).map_err(|e| NCError::Serialize(e))
}

pub fn nc_decode_data<'de, T: Deserialize<'de>>(data: &'de Vec<u8>) -> Result<T, NCError> {
    deserialize(data).map_err(|e| NCError::Deserialize(e))
}