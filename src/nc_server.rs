use std::sync::{Arc, Mutex};
use std::error;
use std::net::{SocketAddr};
use std::time::{Duration, Instant};
use std::collections::HashMap;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{BufReader, BufWriter, AsyncWriteExt};
use tokio::time::{timeout, delay_for};
use tokio::task;

use log::{error, debug};

use serde::{Serialize, Deserialize};

use crate::nc_error::{NCError};
use crate::nc_node::{NCNodeMessage};
use crate::nc_util::{nc_send_message, nc_receive_message, nc_encode_data, nc_decode_data, NCJobStatus};
use crate::nc_config::{NCConfiguration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NCServerMessage {
    HasData(Vec<u8>),
    Waiting,
    Finished,
    HeartBeatMissing,
}

pub trait NCServer {
    fn prepare_data_for_node(&mut self, node_id: u128) -> Result<Vec<u8>, Box<dyn error::Error + Send>>;
    fn process_data_from_node(&mut self, node_id: u128, data: &Vec<u8>) -> Result<(), Box<dyn error::Error + Send>>;
    fn job_status(&self) -> NCJobStatus;
    fn heartbeat_timeout(&mut self, node_id: u128);
}

pub async fn nc_start_server<T: 'static + NCServer + Send>(nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), config.port);
    let socket = TcpListener::bind(addr).await.map_err(|e| NCError::TcpBind(e))?;

    debug!("Listening on: {}", addr);

    let nc_server = Arc::new(Mutex::new(nc_server));
    let connected_nodes: Arc<Mutex<HashMap<u128, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    check_heartbeat(nc_server.clone(), connected_nodes.clone(), config.heartbeat_timeout).await;

    main_loop(nc_server.clone(), connected_nodes.clone(), socket, config.server_timeout).await
}

async fn check_heartbeat<T: 'static + NCServer + Send>(
        nc_server: Arc<Mutex<T>>,
        connected_nodes: Arc<Mutex<HashMap<u128, Instant>>>,
        heartbeat_timeout: u64) {

    tokio::spawn(async move {
        loop {
            delay_for(Duration::from_secs(heartbeat_timeout)).await;

            match nc_server.lock() {
                Ok(mut nc_server) => {
                    if let NCJobStatus::Finished = nc_server.job_status() {
                        debug!("Exit heartbeat loop, job finished!");
                        break
                    } else {
                        match connected_nodes.lock() {
                            Ok(mut connected_nodes) => {
                                let mut dead_nodes = Vec::new();

                                for (node_id, heartbeat_time) in connected_nodes.iter() {
                                    if heartbeat_time.elapsed().as_secs() > heartbeat_timeout {
                                        debug!("Node heartbeat timeout: {}", node_id);
                                        nc_server.heartbeat_timeout(*node_id);
                                        dead_nodes.push(*node_id);
                                    }
                                }

                                for node_id in dead_nodes {
                                    connected_nodes.remove(&node_id);
                                }
                            }
                            Err(e) => {
                                error!("Error in start_server(), heartbeat loop: connected_nodes.lock(): {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error in start_server(), heartbeat loop: nc_server.lock(): {}", e);
                }
            }
        }
    });
}

async fn main_loop<T: 'static + NCServer + Send>(
        nc_server: Arc<Mutex<T>>,
        connected_nodes: Arc<Mutex<HashMap<u128, Instant>>>,
        mut socket: TcpListener,
        server_timeout: u64) -> Result<(), NCError> {

    loop {
        let job_status = nc_server.lock().map_err(|_| NCError::ServerLock)?.job_status();

        match timeout(Duration::from_secs(server_timeout), socket.accept()).await {
            Err(_) => {
                debug!("Received timeout");

                if let NCJobStatus::Finished = job_status {
                    debug!("Job is finished!");
                    // The last node has delivered the last bit of data, so no more nodes will
                    // ever connect to the server again.
                    break
                }
            }
            Ok(Ok((stream, node))) => {
                let nc_server = nc_server.clone();
                let connected_nodes = connected_nodes.clone();

                debug!("Connection from: {}", node.to_string());

                tokio::spawn(async move {
                    match handle_node(nc_server, connected_nodes, stream, job_status).await {
                        Ok(_) => debug!("handle_node() finished"),
                        Err(e) => error!("handle_node() returned an error: {}", e),
                    }
                });
            }
            Ok(Err(e)) => {
                error!("Socket accept error: {}", e);
                return Err(NCError::TcpConnect(e))
            }
        }
    }

    Ok(())
}

async fn handle_node<T: NCServer>(
        nc_server: Arc<Mutex<T>>,
        connected_nodes: Arc<Mutex<HashMap<u128, Instant>>>,
        mut stream: TcpStream,
        job_status: NCJobStatus) -> Result<(), NCError> {

    let (reader, writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut buf_writer = BufWriter::new(writer);

    debug!("Receiving message from node");
    let (num_of_bytes_read, buffer) = nc_receive_message(&mut buf_reader).await?;

    debug!("handle_node(): number of bytes read: {}", num_of_bytes_read);

    match job_status {
        NCJobStatus::Unfinished => {
            debug!("Decoding message");
            match nc_decode_data(&buffer)? {
                NCNodeMessage::NeedsData(node_id) => {
                    debug!("Node needs data: {}", node_id);

                    if heartbeat_received(connected_nodes, node_id)? {
                        let new_data = {
                            let nc_server = &mut nc_server.lock().map_err(|_| NCError::ServerLock)?;

                            debug!("Prepare new data for node");
                            task::block_in_place(|| {
                                nc_server.prepare_data_for_node(node_id).map_err(|e| NCError::ServerPrepare(e))
                            })?
                            // Mutex for nc_server needs to be dropped here
                            // See https://rust-lang.github.io/async-book/07_workarounds/04_send_approximation.html
                        };

                        debug!("Encoding message HasData");
                        let message = nc_encode_data(&NCServerMessage::HasData(new_data))?;
                        let message_length = message.len() as u64;

                        debug!("Sending message to node");
                        nc_send_message(&mut buf_writer, message).await?;

                        debug!("New data sent to node, message_length: {}", message_length);
                    } else {
                        // Node has to send heartbeat first
                        heartbeat_missing(&mut buf_writer).await?;
                    }
                }
                NCNodeMessage::HasData((node_id, new_data)) => {
                    debug!("New processed data received from node: {}", node_id);

                    if heartbeat_received(connected_nodes, node_id)? {
                        let nc_server = &mut nc_server.lock().map_err(|_| NCError::ServerLock)?;

                        debug!("Processing data from node: {}", node_id);
                        task::block_in_place(move || {
                            nc_server.process_data_from_node(node_id, &new_data)
                                .map_err(|e| NCError::ServerProcess(e))
                        })?
                    } else {
                        // Node has to send heartbeat first
                        heartbeat_missing(&mut buf_writer).await?;
                    }
                }
                NCNodeMessage::HeartBeat(node_id) => {
                    debug!("Received heartbeat from node: {}", node_id);

                    let mut connected_nodes = connected_nodes.lock().map_err(|_| NCError::NodesLock)?;
                    connected_nodes.insert(node_id, Instant::now());
                }
            }
        }
        NCJobStatus::Waiting => {
            debug!("Encoding message Waiting");
            let message = nc_encode_data(&NCServerMessage::Waiting)?;

            debug!("Sending message to node");
            nc_send_message(&mut buf_writer, message).await?;

            debug!("Waiting for other nodes to finish");
        }
        NCJobStatus::Finished => {
            debug!("Encoding message Finished");
            let message = nc_encode_data(&NCServerMessage::Finished)?;

            debug!("Sending message to node");
            nc_send_message(&mut buf_writer, message).await?;

            debug!("No more data for node, server has finished");
        }
    }

    Ok(())
}

fn heartbeat_received(connected_nodes: Arc<Mutex<HashMap<u128, Instant>>>, node_id: u128) -> Result<bool, NCError> {
    let connected_nodes = connected_nodes.lock().map_err(|_| NCError::NodesLock)?;
    Ok(connected_nodes.contains_key(&node_id))
}

async fn heartbeat_missing<T: AsyncWriteExt + Unpin>(buf_writer: &mut T)  -> Result<(), NCError> {
    debug!("Encoding message HeartBeatMissing");
    let message = nc_encode_data(&NCServerMessage::HeartBeatMissing)?;

    debug!("Sending message to node");
    nc_send_message(buf_writer, message).await
}
