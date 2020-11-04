use std::{error, fmt, io, net};


#[derive(Debug)]
pub enum NCError {
    IOError(io::Error),
}

impl fmt::Display for NCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NCError::IOError(e) => write!(f, "IOError: {}", e),
        }
    }
}

impl error::Error for NCError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NCError::IOError(e) => Some(e),
        }
    }
}

impl From<io::Error> for NCError {
    fn from(e: io::Error) -> NCError {
        NCError::IOError(e)
    }
}
