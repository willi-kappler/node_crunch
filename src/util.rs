// Std modules
use std::net::{TcpStream};
use std::io::{Write};

// External crates
use serde::{Serialize};
use rmp_serde::{Serializer};

pub fn send_message<M>(stream: &mut TcpStream, message: M)
    where M: Serialize {
    let mut buffer: Vec<u8> = Vec::new();

    match message.serialize(&mut Serializer::new(&mut buffer)) {
        Ok(_) => {
            // Nothing to do for now...
        }
        Err(e) => {
            error!("Could not encode message for client: {:?}", e);
        }
    }

    match stream.write(buffer.as_slice()) {
        Ok(n) => {
            debug!("Number of bytes written: {}", n);
        }
        Err(e) => {
            error!("Could not write to client: {:?}", e);
        }
    }
}
