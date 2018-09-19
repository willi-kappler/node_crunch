// Std modules
use std::net::{TcpStream, SocketAddr};
use std::io::{Read, Write};

// External crates
use failure::Error;
use serde::{Deserialize, Serialize};
use rmp_serde::{Deserializer, Serializer};

// Internal modules
use configuration::{Configuration};
use server::{ServerMessage};

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeMessage {
    ReadyForInput,
    OutputData(u8),
}

pub trait NCNode {
    fn process_new_data_from_server(&mut self, u8) -> u8;
}

pub fn start_node<N>(configuration: Configuration, mut node: N) -> Result<(), Error>
    where N: 'static + NCNode + Send + Sync {

    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    let mut stream = TcpStream::connect(socket)?;
    let mut buffer: Vec<u8> = Vec::new();

    loop {
        // TODO: send ReadyForInput to server at beginning of each loop
        match stream.read_to_end(&mut buffer) {
            Ok(num_of_bytes) => {
                debug!("Number of bytes read: {}", num_of_bytes);
                match handle_message(&mut node, &buffer) {
                    Ok(Some(result)) => {
                        // TODO: send result back to server
                    }
                    Ok(None) => {
                        // No more data -> job finished
                        break
                    }
                    Err(e) => {
                        error!("Could not handle message from server: {:?}", e);
                    }
                }
            }
            Err(e) => {
                error!("Could not read from stream: {:?}", e);
            }
        }
    }

    Ok(())
}

fn handle_message<N>(node: &mut N, buffer: &Vec<u8>) -> Result<Option<u8>, Error>
    where N: NCNode {

    let mut de = Deserializer::new(buffer.as_slice());

    let message: ServerMessage = Deserialize::deserialize(&mut de)?;

    match message {
        ServerMessage::ProcessData(input_data) => {
            let result = node.process_new_data_from_server(input_data);
            Ok(Some(result))
        }
        ServerMessage::JobFinished => {
            Ok(None)
        }
    }
}
