use std::{error, fmt, io, net, sync};

#[derive(Debug)]
pub enum NCError {
    IPAddrParse(net::AddrParseError),
    IOError(io::Error),
    Serialize(bincode::Error),
    Deserialize(bincode::Error),
    ServerMsgMismatch,
    NodeMsgMismatch,
    ThreadJoin,
    MutexPoison,
    Custom(u32),
}

impl fmt::Display for NCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NCError::IPAddrParse(e) => write!(f, "IP address parse error: {}", e),
            NCError::IOError(e) => write!(f, "IO error: {}", e),
            NCError::Serialize(e) => write!(f, "Serialize bincode error: {}", e),
            NCError::Deserialize(e) => write!(f, "Deserialize bincode error: {}", e),
            NCError::ServerMsgMismatch => write!(f, "Server message mismatch error"),
            NCError::NodeMsgMismatch => write!(f, "Node message mismatch error"),
            NCError::ThreadJoin => write!(f, "Error while joining thread"),
            NCError::MutexPoison => write!(f, "Error while locking mutex"),
            NCError::Custom(e) => write!(f, "Custom user defined error: {}", e),
        }
    }
}

impl error::Error for NCError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NCError::IPAddrParse(e) => Some(e),
            NCError::IOError(e) => Some(e),
            NCError::Serialize(e) => Some(e),
            NCError::Deserialize(e) => Some(e),
            NCError::ServerMsgMismatch => None,
            NCError::NodeMsgMismatch => None,
            NCError::ThreadJoin => None,
            NCError::MutexPoison => None,
            NCError::Custom(_) => Some(self),
        }
    }
}

impl From<io::Error> for NCError {
    fn from(e: io::Error) -> NCError {
        NCError::IOError(e)
    }
}

impl From<net::AddrParseError> for NCError {
    fn from(e: net::AddrParseError) -> NCError {
        NCError::IPAddrParse(e)
    }
}

impl<T> From<sync::PoisonError<sync::MutexGuard<'_, T>>> for NCError {
    fn from(_: sync::PoisonError<sync::MutexGuard<'_, T>>) -> NCError {
        NCError::MutexPoison
    }
}
