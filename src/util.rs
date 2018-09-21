// Std modules
use std::net::{TcpStream, Shutdown};
use std::io::{Write};
use std::{time};

// External crates
use serde::{Serialize};
use rmp_serde::{Serializer};

pub fn send_message<M>(stream: &mut TcpStream, message: M)
    where M: Serialize {
    let mut buffer: Vec<u8> = Vec::new();

    match message.serialize(&mut Serializer::new(&mut buffer)) {
        Ok(_) => {
            debug!("Serialize was successfull")
        }
        Err(e) => {
            error!("Could not encode message: {:?}", e);
            buffer.clear();
        }
    }

    match stream.write(buffer.as_slice()) {
        Ok(n) => {
            debug!("Number of bytes written: {}", n);
        }
        Err(e) => {
            error!("Could not write to stream: {:?}", e);
        }
    }
}

pub fn set_timout(stream: &mut TcpStream, timeout: u64) {
    let timeout = Some(time::Duration::from_secs(timeout));

    match stream.set_read_timeout(timeout) {
        Ok(_) => {
            debug!("Set timout successfull");
        }
        Err(e) => {
            error!("Could not set read timeout for stream: {:?}", e);
        }
    }

    match stream.set_write_timeout(timeout) {
        Ok(_) => {
            debug!("Set timout successfull");
        }
        Err(e) => {
            error!("Could not set write timeout for stream: {:?}", e);
        }
    }
}

pub fn shut_down(stream: &mut TcpStream) {
    match stream.shutdown(Shutdown::Both) {
        Ok(_) => {
            debug!("TCP stream shut down successfull");
        }
        Err(e) => {
            error!("Error while shutting down TCP stream: {:?}", e);
        }
    }
}
