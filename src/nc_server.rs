use std::sync::{Arc, Mutex};
use std::error;
use std::net::{SocketAddr};
use std::time::{Duration, Instant};
use std::collections::HashMap;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{BufReader, BufWriter};
use tokio::time::{timeout, delay_for};
use tokio::task;

use log::{error, debug};

use serde::{Serialize, Deserialize};

use crate::nc_error::{NC_Error};
use crate::nc_node::{NC_NodeMessage};
use crate::nc_util::{nc_send_message, nc_receive_message, nc_encode_data, nc_decode_data, NC_JobStatus};
use crate::nc_config::{NC_Configuration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NC_ServerMessage {
    HasData(Vec<u8>),
    Waiting,
    Finished,
}

pub trait NC_Server {
    fn prepare_data_for_node(&mut self, node_id: u128) -> Result<Vec<u8>, Box<dyn error::Error + Send>>;
    fn process_data_from_node(&mut self, node_id: u128, data: &Vec<u8>) -> Result<(), Box<dyn error::Error + Send>>;
    fn job_status(&self) -> NC_JobStatus;
    fn heartbeat_timeout(&mut self, node_id: u128);
}

struct NC_Storage<T> {
    server: T,
    nodes: HashMap<u128, Instant>,
}

pub async fn start_server<T: 'static + NC_Server + Send>(nc_server: T, config: NC_Configuration) -> Result<(), NC_Error> {
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), config.port);
    let mut socket = TcpListener::bind(addr).await.map_err(|e| NC_Error::TcpBind(e))?;

    debug!("Listening on: {}", addr);

    let nc_storage = Arc::new(Mutex::new(NC_Storage{ server: nc_server, nodes: HashMap::new() }));

    let nc_storage_local = nc_storage.clone();

    // Check heartbeat
    tokio::spawn(async move {
        loop {
            /*
            delay_for(Duration::from_secs(config.heartbeat_timeout)).await;
            
            let nc_storage = &mut nc_storage_local.lock().unwrap();

            if let NC_JobStatus::Finished = nc_storage.server.job_status() {
                debug!("Exit heartbeat loop, job finished!");
                break
            } else {
                for (node_id, heartbeat_time) in nc_storage.nodes {
                    if heartbeat_time.elapsed().as_secs() > config.heartbeat_timeout {
                        debug!("Node heartbeat timeout: {}", node_id);
                        nc_storage.server.heartbeat_timeout(node_id);
                    }
                }
            }
            */
        }
    });

    // Main server loop
    loop {
        let job_status = nc_storage.lock().map_err(|_| NC_Error::StorageLock)?.server.job_status();

        match timeout(Duration::from_secs(config.server_timeout), socket.accept()).await {
            Err(_) => {
                debug!("Received timeout");

                if let NC_JobStatus::Finished = job_status {
                    debug!("Job is finished!");
                    // The last node has delivered tha last bit of data, so no more nodes will
                    // ever connect to the server again.
                    break
                }
            }
            Ok(Ok((stream, node))) => {
                let nc_storage_local = nc_storage.clone();
        
                debug!("Connection from: {}", node.to_string());
        
                tokio::spawn(async move {
                    match handle_node(nc_storage_local, stream, job_status).await {
                        Ok(_) => debug!("handle node finished"),
                        Err(e) => error!("handle node returned an error: {}", e),
                    }
                });
            }
            Ok(Err(e)) => {
                error!("Socket accept error: {}", e);
                return Err(NC_Error::TcpConnect(e))
            }
        }

    }

    Ok(())
}

async fn handle_node<T: NC_Server>(nc_storage: Arc<Mutex<NC_Storage<T>>>, mut stream: TcpStream, job_status: NC_JobStatus) -> Result<(), NC_Error> {
    let (reader, writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut buf_writer = BufWriter::new(writer);
    
    debug!("Receiving message from node");
    let (num_of_bytes_read, buffer) = nc_receive_message(&mut buf_reader).await?;

    debug!("handle_node: number of bytes read: {}", num_of_bytes_read);

    match job_status {
        NC_JobStatus::Unfinished => {
            debug!("Decoding message");
            match nc_decode_data(&buffer)? {
                NC_NodeMessage::NeedsData(node_id) => {
                    debug!("Node needs data: {}", node_id);
                    let new_data = {
                        let nc_server = &mut nc_storage.lock().map_err(|_| NC_Error::StorageLock)?.server;
    
                        debug!("Prepare new data for node");
                        task::block_in_place(move || {
                            nc_server.prepare_data_for_node(node_id).map_err(|e| NC_Error::ServerPrepare(e))
                        })?
                        // Mutex for nc_storage needs to be dropped here
                        // See https://rust-lang.github.io/async-book/07_workarounds/04_send_approximation.html
                    }; 
    
                    debug!("Encoding message HasData");
                    let message = nc_encode_data(&NC_ServerMessage::HasData(new_data))?;
                    let message_length = message.len() as u64;
    
                    debug!("Sending message to node");
                    nc_send_message(&mut buf_writer, message).await?;
        
                    debug!("New data sent to node, message_length: {}", message_length);
                }
                NC_NodeMessage::HasData((node_id, new_data)) => {
                    debug!("New processed data received from node: {}", node_id);
                    let nc_server = &mut nc_storage.lock().map_err(|_| NC_Error::StorageLock)?.server;

                    debug!("Processing data from node: {}", node_id);
                    task::block_in_place(move || {
                        nc_server.process_data_from_node(node_id, &new_data)
                            .map_err(|e| NC_Error::ServerProcess(e))
                    })?
                }
                NC_NodeMessage::HeartBeat(node_id) => {
                    debug!("Received heart beat from node: {}", node_id);

                    let mut nc_storage = nc_storage.lock().map_err(|_| NC_Error::StorageLock)?;
                    nc_storage.nodes.insert(node_id, Instant::now());
                }
            }        
        }
        NC_JobStatus::Waiting => {
            debug!("Encoding message Waiting");
            let message = nc_encode_data(&NC_ServerMessage::Waiting)?;

            debug!("Sending message to node");
            nc_send_message(&mut buf_writer, message).await?;

            debug!("Waiting for other nodes to finish");
        }
        NC_JobStatus::Finished => {
            debug!("Encoding message Finished");
            let message = nc_encode_data(&NC_ServerMessage::Finished)?;

            debug!("Sending message to node");
            nc_send_message(&mut buf_writer, message).await?;

            debug!("No more data for node, server has finished");
        }
    }

    Ok(())
}
