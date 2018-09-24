// Std modules
use std::net::{TcpStream, SocketAddr};
use std::io::{Read};

// External crates
use failure::Error;
use serde::{Serialize, de::DeserializeOwned};
use bincode::{deserialize};

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
    fn process_new_data_from_server(&mut self, input: T) -> U;
}

pub fn start_node<N, T, U>(configuration: Configuration, mut node: N) -> Result<(), Error>
    where N: 'static + NCNode<T, U> + Send + Sync,
          T: DeserializeOwned,
          U: Serialize {

    let mut buffer: Vec<u8> = vec![0; 1024];
    let mut data_processed: Option<U> = None;

    debug!("Enter node loop");

    loop {
        let mut stream = connect_to_server(&configuration)?;

        match &data_processed {
            None => {
                debug!("Send ready for input to server");
                send_message(&mut stream, NodeMessage::ReadyForInput::<U>);
            }
            Some(result) => {
                debug!("Send result to server");
                send_message(&mut stream, NodeMessage::OutputData(result));
            }
        }

        data_processed = None;

        debug!("Waiting for data from server...");

        match stream.read(&mut buffer) {
            Ok(num_of_bytes) => {
                debug!("Number of bytes read: {}", num_of_bytes);
                match handle_message(&mut node, &buffer) {
                    Ok(Some(result)) => {
                        debug!("Data has been processed");
                        data_processed = Some(result);
                    }
                    Ok(None) => {
                        debug!("No more data to process, job done!");
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

        debug!("Shut down connection");
        debug!("--------------------");
        // shut_down(&mut stream);
    }

    Ok(())
}

fn connect_to_server(configuration: &Configuration) -> Result<TcpStream, Error> {
    debug!("Connect to server");
    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    // TODO: retry multiple times...
    let stream = TcpStream::connect(socket)?;
    // set_timout(&mut stream, configuration.timeout);
    Ok(stream)
}

fn handle_message<N, T, U>(node: &mut N, buffer: &Vec<u8>) -> Result<Option<U>, Error>
    where N: NCNode<T, U>,
          T: DeserializeOwned {

    debug!("Handle message on node");

    let message = deserialize(buffer)?;

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
