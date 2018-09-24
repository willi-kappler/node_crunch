// Std modules
use std::net::{TcpListener, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::marker::{Sync, Send};
use std::{thread, time};
use std::io::{Read};

// External crates
use failure::Error;
use serde::{Serialize, de::DeserializeOwned};
use bincode::{deserialize};

// Internal modules
use configuration::{Configuration};
use node::{NodeMessage};
use util::{send_message};

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage<T> {
    JobFinished,
    ProcessData(T),
}

pub trait NCServer<T, U> {
    fn prepare_node_input(&mut self) -> T;

    fn process_node_output(&mut self, result: U);

    fn is_job_done(&mut self) -> bool;
}

pub fn start_server<S, T, U>(configuration: Configuration, server: S) -> Result<(), Error>
    where S: 'static + NCServer<T, U> + Send + Sync,
          T: Serialize,
          U: DeserializeOwned {

    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    let listener = TcpListener::bind(socket)?;
    let main_server = Arc::new(Mutex::new(server));
    let mut thread_handlers = Vec::new();

    loop {
        debug!("Start loop, waiting for nodes to connect...");
        match main_server.lock() {
            Ok(mut server) => {
                if server.is_job_done() {
                    debug!("Job is done!");
                    break
                }
            }
            Err(e) => {
                error!("Mutex error: {:?}", e);
            }
        }

        debug!("Waiting for data from node...");

        match listener.accept() {
            Ok((mut stream, address)) => {
                debug!("Connection from node: {}", address);
                // set_timout(&mut stream, configuration.timeout);

                let local_server = main_server.clone();
                thread_handlers.push(thread::spawn(move || {
                    debug!("New thread started");
                    let mut buffer: Vec<u8> = vec![0; 1024];
                    match local_server.lock() {
                        Ok(mut server) => {
                            debug!("Reading data from node...");
                            match stream.read(&mut buffer) {
                                Ok(num_of_bytes) => {
                                    debug!("Number of bytes read: {}", num_of_bytes);
                                    handle_message(&mut *server, &buffer, &mut stream);
                                }
                                Err(e) => {
                                    error!("Could not read from TcpStream: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Mutex error: {:?}", e);
                        }
                    }
                    debug!("Shutting down connection");
                    debug!("--------------------");
                }));
            }
            Err(e) => {
                error!("Could not accept node connecton: {:?}", e);
            }
        }
    }

    let wait_to_finish = time::Duration::from_secs(5);

    thread::sleep(wait_to_finish);

    for thread in thread_handlers {
        match thread.join() {
            Ok(_) => {
                debug!("Thread joined successfully");
            }
            Err(e) => {
                error!("Could not join thread: {:?}", e);
            }
        }
    }

    Ok(())
}

fn handle_message<S, T, U>(server: &mut S, buffer: &Vec<u8>, stream: &mut TcpStream)
    where S: NCServer<T, U>,
          T: Serialize,
          U: DeserializeOwned {

    let data = deserialize(buffer);

    match data {
        Ok(message) => {
            match message {
                NodeMessage::ReadyForInput => {
                    debug!("Node is ready for more input.");
                }
                NodeMessage::OutputData(result) => {
                    debug!("Node has delivered some results.");
                    server.process_node_output(result);
                }
            }

            if server.is_job_done() {
                debug!("Job is done, no more data to process");
                send_message(stream, ServerMessage::JobFinished::<T>);
            } else {
                debug!("Send some data to node");
                let node_input_data = server.prepare_node_input();
                send_message(stream, ServerMessage::ProcessData(node_input_data));
            }
        }
        Err(e) => {
            error!("Could not deserialize message: {:?}", e);
        }
    }
}
