//! This module contains the common error type for server and node.

use std::{error, fmt, io, net, sync};

use crate::{NodeID, array2d::Array2DError};

/// This data structure contains all error codes for the server and the nodes.
#[derive(Debug)]
pub enum NCError {
    /// Parsing the IP address went wrong.
    IPAddrParse(net::AddrParseError),
    /// Common IO error, usually network related.
    IOError(io::Error),
    /// Data could not be serialized for sending over the network.
    Serialize(bincode::Error),
    /// Data coming from the network could not be deserialized.
    Deserialize(bincode::Error),
    /// The bincode crate has its own error.
    Bincode(Box<bincode::ErrorKind>),
    /// The node expected a specific message from the server but got s.th. totally different.
    ServerMsgMismatch,
    /// The server expected a specific message from the node but got s.th. totally different.
    NodeMsgMismatch,
    /// A different node id was expected. Expected first node id, found second node id.
    NodeIDMismatch(NodeID, NodeID),
    /// Mutex could not be locked or a thread paniced while holding the lock.
    MutexPoison,
    /// An error using the utility data structure Array2D.
    Array2D(Array2DError),
    /// Custom user defined error. This needs to be replaced in the future with Box<dyn Error> or s.th. similar.
    Custom(u32),
}

impl fmt::Display for NCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NCError::IPAddrParse(e) => write!(f, "IP address parse error: {}", e),
            NCError::IOError(e) => write!(f, "IO error: {}", e),
            NCError::Serialize(e) => write!(f, "Serialize bincode error: {}", e),
            NCError::Deserialize(e) => write!(f, "Deserialize bincode error: {}", e),
            NCError::Bincode(e) => write!(f, "Bincode error: {}", e),
            NCError::ServerMsgMismatch => write!(f, "Server message mismatch error"),
            NCError::NodeMsgMismatch => write!(f, "Node message mismatch error"),
            NCError::NodeIDMismatch(id1, id2) => write!(f, "Node id mismatch error, expected: {}, found: {}", id1, id2),
            NCError::MutexPoison => write!(f, "Mutex poisson error"),
            NCError::Array2D(e) => write!(f, "Array2D error: {}", e),
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
            NCError::Bincode(e) => Some(e),
            NCError::ServerMsgMismatch => None,
            NCError::NodeMsgMismatch => None,
            NCError::NodeIDMismatch(_, _) => None,
            NCError::MutexPoison => None,
            NCError::Array2D(e) => Some(e),
            NCError::Custom(_) => Some(self),
        }
    }
}

impl From<io::Error> for NCError {
    fn from(e: io::Error) -> NCError {
        NCError::IOError(e)
    }
}

impl From<Box<bincode::ErrorKind>> for NCError {
    fn from(e: Box<bincode::ErrorKind>) -> NCError {
        NCError::Bincode(e)
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

