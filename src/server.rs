// Std modules
use std::net::{TcpListener, SocketAddr};
use std::sync::{Arc, Mutex};
use std::marker::{Sync, Send};
use std::{thread, time};
use std::io::{Read};

// External crates
use failure::Error;
use serde::{Deserialize, Serialize};
use rmp_serde::{Deserializer, Serializer};

// Internal modules
use configuration::{Configuration};
use client::{NodeMessage};

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    ProcessData,
    JobFinished,
}

pub trait NCServer {
    fn prepare_node_input(&mut self);

    fn process_node_result(&mut self);

    fn is_job_done(&mut self) -> bool;
}


pub fn start_server<S>(configuration: Configuration, server: S) -> Result<(), Error>
    where S: 'static + NCServer + Send + Sync {

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
                let timeout = Some(time::Duration::from_secs(5));
                match stream.set_read_timeout(timeout) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Could not set read timeout for stream: {:?}", e);
                    }
                }

                match stream.set_write_timeout(timeout) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Could not set write timeout for stream: {:?}", e);
                    }
                }

                let local_server = main_server.clone();
                thread_handlers.push(thread::spawn(move || {
                    let mut buffer: Vec<u8> = Vec::new();
                    loop {
                        buffer.clear();
                        match local_server.lock() {
                            Ok(mut server) => {
                                if server.is_job_done() {
                                    return
                                } else {
                                    match stream.read_to_end(&mut buffer) {
                                        Ok(num_of_bytes) => {
                                            debug!("Number of bytes read: {}", num_of_bytes);
                                            // TODO: Deserialize buffer and match on message type
                                            match handle_message(&mut *server, &buffer) {
                                                Ok(_) => {

                                                }
                                                Err(e) => {
                                                    error!("Error handling message: {:?}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("Could not read from TcpStream: {:?}", e)
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
                warn!("Could not accept node connecton: {:?}", e)
            }
        }
    }

    let wait_to_finish = time::Duration::from_secs(10);

    thread::sleep(wait_to_finish);

    Ok(())
}

fn handle_message<S>(server: &mut S, buffer: &Vec<u8>) -> Result<(), Error>
    where S: NCServer {

    let mut de = Deserializer::new(buffer.as_slice());

    let message: NodeMessage = Deserialize::deserialize(&mut de)?;

    match message {
        NodeMessage::ReadyForInput => {
            server.prepare_node_input();
        }
        NodeMessage::OutputData => {
            server.process_node_result();
        }
    }

    Ok(())
}
