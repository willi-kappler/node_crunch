use std::{error, fmt, io};

use bincode;

#[derive(Debug)]
pub enum NCError {
    IOError(io::Error),
    Serialize(bincode::Error),
    Deserialize(bincode::Error),
    Custom(u32),
}

impl fmt::Display for NCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NCError::IOError(e) => write!(f, "IOError: {}", e),
            NCError::Serialize(e) => write!(f, "Serialize bincode error: {}", e),
            NCError::Deserialize(e) => write!(f, "Deserialize bincode error: {}", e),
            NCError::Custom(e) => write!(f, "Custom user defined error: {}", e),
        }
    }
}

impl error::Error for NCError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NCError::IOError(e) => Some(e),
            NCError::Serialize(e) => Some(e),
            NCError::Deserialize(e) => Some(e),
            NCError::Custom(_) => Some(self),
        }
    }
}

impl From<io::Error> for NCError {
    fn from(e: io::Error) -> NCError {
        NCError::IOError(e)
    }
}
