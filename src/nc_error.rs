use std::{error, fmt, io, net};

use bincode;

#[derive(Debug)]
pub enum NCError {
    IPAddr(net::AddrParseError),
    TcpBind(io::Error),
    TcpConnect(io::Error),
    SocketAccept(io::Error),
    ReadU64(io::Error),
    ReadBuffer(io::Error),
    QuitLock,
    ServerLock,
    NodesLock,
    WriteU64(io::Error),
    WriteBuffer(io::Error),
    Serialize(bincode::Error),
    Deserialize(bincode::Error),
    NodeProcess(Box<dyn error::Error + Send>),
    ServerPrepare(Box<dyn error::Error + Send>),
    ServerProcess(Box<dyn error::Error + Send>),
}

impl fmt::Display for NCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NCError::IPAddr(e) => write!(f, "IPAddr error: {}", e),
            NCError::TcpBind(e) => write!(f, "TcpBind error: {}", e),
            NCError::TcpConnect(e) => write!(f, "TcpConnect error: {}", e),
            NCError::SocketAccept(e) => write!(f, "SocketAccept error: {}", e),
            NCError::ReadU64(e) => write!(f, "ReadU64 error: {}", e),
            NCError::ReadBuffer(e) => write!(f, "ReadBuffer error: {}", e),
            NCError::QuitLock => write!(f, "QuitLock error"),
            NCError::ServerLock => write!(f, "ServerLock error"),
            NCError::NodesLock => write!(f, "NodesLock error"),
            NCError::WriteU64(e) => write!(f, "WriteU64 error: {}", e),
            NCError::WriteBuffer(e) => write!(f, "WriteBuffer error: {}", e),
            NCError::Serialize(e) => write!(f, "Serialize error: {}", e),
            NCError::Deserialize(e) => write!(f, "Deserialize error: {}", e),
            NCError::NodeProcess(e) => write!(f, "NodeProcess error: {}", e),
            NCError::ServerPrepare(e) => write!(f, "ServerPrepare error: {}", e),
            NCError::ServerProcess(e) => write!(f, "ServerProcess error: {}", e),
        }
    }
}

impl error::Error for NCError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NCError::IPAddr(e) => Some(e),
            NCError::TcpBind(e) => Some(e),
            NCError::TcpConnect(e) => Some(e),
            NCError::SocketAccept(e) => Some(e),
            NCError::ReadU64(e) => Some(e),
            NCError::ReadBuffer(e) => Some(e),
            NCError::QuitLock => None,
            NCError::ServerLock => None,
            NCError::NodesLock => None,
            NCError::WriteU64(e) => Some(e),
            NCError::WriteBuffer(e) => Some(e),
            NCError::Serialize(e) => Some(e),
            NCError::Deserialize(e) => Some(e),
            NCError::NodeProcess(_e) => None, // Some(e) doesn't work
            NCError::ServerPrepare(_e) => None, // Some(e) doesn't work
            NCError::ServerProcess(_e) => None, // Some(e) doesn't work
        }
    }
}
