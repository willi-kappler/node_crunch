use std::{error, fmt, io, net};

use bincode;

#[derive(Debug)]
pub enum NC_Error {
    IPAddr(net::AddrParseError),
    TcpBind(io::Error),
    TcpConnect(io::Error),
    SocketAccept(io::Error),
    ReadU64(io::Error),
    ReadBuffer(io::Error),
    QuitLock,
    ServerLock,
    WriteU64(io::Error),
    WriteBuffer(io::Error),
    Serialize(bincode::Error),
    Deserialize(bincode::Error),
    NodeProcess(Box<dyn error::Error + Send>),
    ServerPrepare(Box<dyn error::Error + Send>),
    ServerProcess(Box<dyn error::Error + Send>),
}

impl fmt::Display for NC_Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NC_Error::IPAddr(e) => write!(f, "IPAddr error: {}", e),
            NC_Error::TcpBind(e) => write!(f, "TcpBind error: {}", e),
            NC_Error::TcpConnect(e) => write!(f, "TcpConnect error: {}", e),
            NC_Error::SocketAccept(e) => write!(f, "SocketAccept error: {}", e),
            NC_Error::ReadU64(e) => write!(f, "ReadU64 error: {}", e),
            NC_Error::ReadBuffer(e) => write!(f, "ReadBuffer error: {}", e),
            NC_Error::QuitLock => write!(f, "QuitLock error"),
            NC_Error::ServerLock => write!(f, "ServerLock error"),
            NC_Error::WriteU64(e) => write!(f, "WriteU64 error: {}", e),
            NC_Error::WriteBuffer(e) => write!(f, "WriteBuffer error: {}", e),
            NC_Error::Serialize(e) => write!(f, "Serialize error: {}", e),
            NC_Error::Deserialize(e) => write!(f, "Deserialize error: {}", e),
            NC_Error::NodeProcess(e) => write!(f, "NodeProcess error: {}", e),
            NC_Error::ServerPrepare(e) => write!(f, "ServerPrepare error: {}", e),
            NC_Error::ServerProcess(e) => write!(f, "ServerProcess error: {}", e),
        }        
    }
}

impl error::Error for NC_Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NC_Error::IPAddr(e) => Some(e),
            NC_Error::TcpBind(e) => Some(e),
            NC_Error::TcpConnect(e) => Some(e),
            NC_Error::SocketAccept(e) => Some(e),
            NC_Error::ReadU64(e) => Some(e),
            NC_Error::ReadBuffer(e) => Some(e),
            NC_Error::QuitLock => None,
            NC_Error::ServerLock => None,
            NC_Error::WriteU64(e) => Some(e),
            NC_Error::WriteBuffer(e) => Some(e),
            NC_Error::Serialize(e) => Some(e),
            NC_Error::Deserialize(e) => Some(e),
            NC_Error::NodeProcess(e) => None, // Some(e) doesn't work
            NC_Error::ServerPrepare(e) => None, // Some(e) doesn't work
            NC_Error::ServerProcess(e) => None, // Some(e) doesn't work
        }
    }
}
