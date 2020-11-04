use std::{error, fmt, io, net};

#[derive(Debug)]
pub enum NCError {
    SomeError
}

impl fmt::Display for NCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NCError::SomeError => write!(f, "SomeError"),
        }
    }
}

impl error::Error for NCError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NCError::SomeError => None,
        }
    }
}
