use std::{error, fmt, io};
use bincode;

#[derive(Debug)]
pub enum NCError {
    TcpBind(io::Error),
    SocketAccept(io::Error),
    ReadU64(io::Error),
    ReadBuffer(io::Error),
    QuitLock,
    ServerLock,
    WriteU64(io::Error),
    WriteBuffer(io::Error),
    Serialize(bincode::Error),
    Deserialize(bincode::Error),
}

impl fmt::Display for NCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NCError::TcpBind(e) => write!(f, "TcpBind error: {}", e),
            NCError::SocketAccept(e) => write!(f, "SocketAccept error: {}", e),
            NCError::ReadU64(e) => write!(f, "ReadU64 error: {}", e),
            NCError::ReadBuffer(e) => write!(f, "ReadBuffer error: {}", e),
            NCError::QuitLock => write!(f, "QuitLock error"),
            NCError::ServerLock => write!(f, "ServerLock error"),
            NCError::WriteU64(e) => write!(f, "WriteU64 error: {}", e),
            NCError::WriteBuffer(e) => write!(f, "WriteBuffer error: {}", e),
            NCError::Serialize(e) => write!(f, "Serialize error: {}", e),
            NCError::Deserialize(e) => write!(f, "Deserialize error: {}", e),
        }        
    }
}

impl error::Error for NCError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NCError::TcpBind(e) => Some(e),
            NCError::SocketAccept(e) => Some(e),
            NCError::ReadU64(e) => Some(e),
            NCError::ReadBuffer(e) => Some(e),
            NCError::QuitLock => None,
            NCError::ServerLock => None,
            NCError::WriteU64(e) => Some(e),
            NCError::WriteBuffer(e) => Some(e),
            NCError::Serialize(e) => Some(e),
            NCError::Deserialize(e) => Some(e),
        }
    }
}
