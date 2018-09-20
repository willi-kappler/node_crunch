// Std modules
use std::net::{TcpStream, SocketAddr, Shutdown};
use std::io::{Read};

// External crates
use failure::Error;
use serde::{Serialize, Deserialize};
use rmp_serde::{Deserializer};

// Internal modules
use configuration::{Configuration};
use server::{ServerMessage};
use util::{send_message};

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeMessage<U> {
    ReadyForInput,
    OutputData(U),
}

pub trait NCNode<T, U> {
    fn process_new_data_from_server(&mut self, T) -> U;
}

pub fn start_node<'a, N, T, U>(configuration: Configuration, mut node: N) -> Result<(), Error>
    where N: 'static + NCNode<T, U> + Send + Sync,
          T: Deserialize<'a>,
          U: Serialize {

    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    let mut stream = TcpStream::connect(socket)?;
    let mut buffer: Vec<u8> = Vec::new();

    loop {
        send_message(&mut stream, NodeMessage::ReadyForInput::<U>);

        match stream.read_to_end(&mut buffer) {
            Ok(num_of_bytes) => {
                debug!("Number of bytes read: {}", num_of_bytes);
                match handle_message(&mut node, &buffer) {
                    Ok(Some(result)) => {
                        send_message(&mut stream, NodeMessage::OutputData(result));
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

    match stream.shutdown(Shutdown::Both) {
        Ok(_) => {
            // Nothing to do for now...
        }
        Err(e) => {
            error!("Error while shutting down TCP stream: {:?}", e);
        }
    }

    Ok(())
}

fn handle_message<'a, N, T, U>(node: &mut N, buffer: &Vec<u8>) -> Result<Option<U>, Error>
    where N: NCNode<T, U>,
          T: Deserialize<'a> {

    let mut de = Deserializer::new(buffer.as_slice());
    let message: ServerMessage<T> = Deserialize::deserialize(&mut de)?;

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
