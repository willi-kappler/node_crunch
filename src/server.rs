// Std modules
use std::net::{TcpListener, SocketAddr, Shutdown};
use std::sync::{Arc, Mutex};
use std::marker::{Sync, Send};
use std::{thread, time};
use std::io::{Read};

// External crates
use failure::Error;
use serde::{Deserialize, Serialize};
use rmp_serde::{Deserializer};

// Internal modules
use configuration::{Configuration};
use node::{NodeMessage};
use util::{send_message, set_timout};

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage<T> {
    ProcessData(T),
    JobFinished,
}

pub trait NCServer<T, U> {
    fn prepare_node_input(&mut self) -> T;

    fn process_node_output(&mut self, result: U);

    fn is_job_done(&mut self) -> bool;
}

pub fn start_server<'a, S, T, U>(configuration: Configuration, server: S) -> Result<(), Error>
    where S: 'static + NCServer<T, U> + Send + Sync,
          T: Serialize,
          U: Deserialize<'a> {

    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    let listener = TcpListener::bind(socket)?;
    let main_server = Arc::new(Mutex::new(server));
    let mut thread_handlers = Vec::new();

    loop {
        match main_server.lock() {
            Ok(mut server) => {
                if server.is_job_done() {
                    break
                }
            }
            Err(e) => {
                error!("Mutex error: {:?}", e);
            }
        }

        match listener.accept() {
            Ok((mut stream, address)) => {
                info!("Connection from node: {}", address);
                set_timout(&mut stream);

                let local_server = main_server.clone();
                thread_handlers.push(thread::spawn(move || {
                    let mut buffer: Vec<u8> = Vec::new();
                    loop {
                        buffer.clear();
                        match local_server.lock() {
                            Ok(mut server) => {
                                if server.is_job_done() {
                                    send_message(&mut stream, ServerMessage::JobFinished::<T>);
                                    match stream.shutdown(Shutdown::Both) {
                                        Ok(_) => {
                                            // Nothing to do for now...
                                        }
                                        Err(e) => {
                                            error!("Error while shutting down TCP stream: {:?}", e);
                                        }
                                    }

                                    return
                                } else {
                                    match stream.read_to_end(&mut buffer) {
                                        Ok(num_of_bytes) => {
                                            debug!("Number of bytes read: {}", num_of_bytes);
                                            match handle_message(&mut *server, &buffer) {
                                                Ok(Some(node_input_data)) => {
                                                    // Send input data to node, so that it can start processing
                                                    send_message(&mut stream, ServerMessage::ProcessData(node_input_data));
                                                }
                                                Ok(None) => {
                                                    // Nothing to do for now...
                                                }
                                                Err(e) => {
                                                    error!("Error handling message: {:?}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("Could not read from TcpStream: {:?}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Mutex error: {:?}", e);
                            }
                        }
                    }
                }));

            }
            Err(e) => {
                error!("Could not accept node connecton: {:?}", e);
            }
        }
    }

    let wait_to_finish = time::Duration::from_secs(10);

    thread::sleep(wait_to_finish);

    for thread in thread_handlers {
        match thread.join() {
            Ok(_) => {
                // Nothing to do for now
            }
            Err(e) => {
                error!("Could not join thread: {:?}", e);
            }
        }
    }

    Ok(())
}

fn handle_message<'a, S, T, U>(server: &mut S, buffer: &Vec<u8>) -> Result<Option<T>, Error>
    where S: NCServer<T, U>,
          T: Serialize,
          U: Deserialize<'a> {

    let mut de = Deserializer::new(buffer.as_slice());
    let message: NodeMessage<U> = Deserialize::deserialize(&mut de)?;

    match message {
        NodeMessage::ReadyForInput => {
            Ok(Some(server.prepare_node_input()))
        }
        NodeMessage::OutputData(result) => {
            server.process_node_output(result);
            Ok(None)
        }
    }
}
