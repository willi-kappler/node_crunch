// Std modules
use std::net::{TcpStream, SocketAddr};
use std::io::{Read};

// External crates
use failure::Error;
use serde::{Serialize, Deserialize};
use rmp_serde::{Deserializer};

// Internal modules
use configuration::{Configuration};
use server::{ServerMessage};
use util::{send_message, set_timout, shut_down};

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

    let mut buffer: Vec<u8> = Vec::new();
    let mut data_processed: Option<U> = None;

    debug!("Enter node loop");

    loop {
        buffer.clear();
        let mut stream = connect_to_server(&configuration)?;

        match &data_processed {
            None => {
                send_message(&mut stream, NodeMessage::ReadyForInput::<U>);
            }
            Some(result) => {
                debug!("Send result to server");
                send_message(&mut stream, NodeMessage::OutputData(result));
            }
        }

        data_processed = None;

        match stream.read_to_end(&mut buffer) {
            Ok(num_of_bytes) => {
                debug!("Number of bytes read: {}", num_of_bytes);
                match handle_message(&mut node, &buffer) {
                    Ok(Some(result)) => {
                        debug!("Data has been processed");
                        data_processed = Some(result);
                    }
                    Ok(None) => {
                        debug!("No more data to process, job done!");
                        shut_down(&mut stream);
                        break;
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

        shut_down(&mut stream);
    }

    Ok(())
}

fn connect_to_server(configuration: &Configuration) -> Result<TcpStream, Error> {
    debug!("Connect to server");
    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    // TODO: retry multiple times...
    let mut stream = TcpStream::connect(socket)?;
    set_timout(&mut stream, configuration.timeout);
    Ok(stream)
}

fn handle_message<'a, N, T, U>(node: &mut N, buffer: &Vec<u8>) -> Result<Option<U>, Error>
    where N: NCNode<T, U>,
          T: Deserialize<'a> {

    debug!("Handle message on node");

    let mut de = Deserializer::new(buffer.as_slice());
    let message: ServerMessage<T> = Deserialize::deserialize(&mut de)?;

    match message {
        ServerMessage::JobFinished => {
            debug!("Received job done from server");
            Ok(None)
        }
        ServerMessage::ProcessData(input_data) => {
            debug!("Received data from server, processing...");
            let result = node.process_new_data_from_server(input_data);
            Ok(Some(result))
        }
    }
}
