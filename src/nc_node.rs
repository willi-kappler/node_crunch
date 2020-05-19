use std::time::Duration;
use std::net::{SocketAddr};

use tokio::net::TcpStream;
use tokio::io::{BufReader, BufWriter};
use tokio::time::delay_for;
use tokio::task;

use log::{error, debug};

use serde::{Serialize, Deserialize};

use rand::{self, Rng};

use crate::nc_error::{NCError};
use crate::nc_server::{NCServerMessage};
use crate::nc_util::{nc_send_message, nc_receive_message, nc_encode_data, nc_decode_data};
use crate::nc_config::{NCConfiguration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NCNodeMessage {
    NeedsData(u128),
    HasData((u128, Vec<u8>)),
    HeartBeat(u128),
}

pub trait NCNode {
    fn process_data_from_server(&mut self, data: Vec<u8>) -> Vec<u8>;
}

pub async fn nc_start_node<T: NCNode>(mut nc_node: T, config: NCConfiguration) -> Result<(), NCError> {
    let addr = SocketAddr::new(config.address.parse().map_err(|e| NCError::IPAddr(e))?, config.port);

    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes[..]);
    let node_id: u128 = u128::from_le_bytes(bytes);

    debug!("Current node id: {}", node_id);

    send_heartbeat(&addr, config.heartbeat_timeout / 2, node_id).await?;
    main_loop(&mut nc_node, &addr, config.reconnect_wait, node_id).await
}

async fn send_heartbeat(addr: &SocketAddr, heartbeat_time: u64, node_id: u128) -> Result<(), NCError> {
    debug!("Connecting to server: {}", addr);
    let addr = addr.clone();

    tokio::spawn(async move {
        loop {
            match TcpStream::connect(&addr).await {
                Ok(mut stream) => {
                    let (_, writer) = stream.split();
                    let mut buf_writer = BufWriter::new(writer);

                    debug!("Encoding message HeartBeat");
                    match nc_encode_data(&NCNodeMessage::HeartBeat(node_id)) {
                        Ok(message) => {
                            debug!("Sending message to server");
                            match nc_send_message(&mut buf_writer, message).await {
                                Ok(_) => {
                                    debug!("HeartBeat sent");
                                }
                                Err(e) => {
                                    error!("send_heartbeat(), nc_send_message(): an error occurred: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("send_heartbeat(), nc_encode_data(): an error occurred: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("send_heartbeat(), TcpStream::connect(): an error occurred: {}", e);
                }
            }

            delay_for(Duration::from_secs(heartbeat_time)).await;
        }
    });

    Ok(())
}

async fn main_loop<T: NCNode>(nc_node: &mut T, addr: &SocketAddr, reconnect_wait: u64, node_id: u128) -> Result<(), NCError> {
    loop {
        match send_node_needs_data(&addr, node_id).await {
            Ok(NCServerMessage::HasData(data)) => {
                debug!("Received HasData");
                debug!("Processing data...");
                let processed_data = task::block_in_place(|| { nc_node.process_data_from_server(data) });

                match send_node_has_data(&addr, processed_data, node_id).await {
                    Ok(_) => {
                        debug!("Node has data sent");
                    }
                    Err(e) => {
                        error!("An error occurred: {}", e);
                        debug!("Will retry again");
                    }
                }
            }
            Ok(NCServerMessage::Waiting) => {
                debug!("Retry in {} seconds", reconnect_wait);
                delay_for(Duration::from_secs(reconnect_wait)).await;
            }
            Ok(NCServerMessage::Finished) => {
                debug!("Job is finished, exit loop");
                break
            }
            Ok(NCServerMessage::HeartBeatMissing) => {
                debug!("Heartbeat is missing from this node: {}", node_id);
                delay_for(Duration::from_secs(reconnect_wait)).await;
            }
            Err(e) => {
                error!("An error occurred: {}", e);

                debug!("Retry in {} seconds", reconnect_wait);
                delay_for(Duration::from_secs(reconnect_wait)).await;
            }
        }
    }

    Ok(())
}

async fn send_node_needs_data(addr: &SocketAddr, node_id: u128) -> Result<NCServerMessage, NCError> {
    debug!("Connecting to server: {}", addr);
    let mut stream = TcpStream::connect(&addr).await.map_err(|e| NCError::TcpConnect(e))?;
    let (reader, writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut buf_writer = BufWriter::new(writer);

    debug!("Encoding message NeedsData");
    let message = nc_encode_data(&NCNodeMessage::NeedsData(node_id))?;

    debug!("Sending message to server");
    nc_send_message(&mut buf_writer, message).await?;

    debug!("Receiving message from server");
    let (num_of_bytes_read, buffer) = nc_receive_message(&mut buf_reader).await?;

    debug!("Number of bytes read: {}", num_of_bytes_read);
    debug!("Decoding message");
    nc_decode_data(&buffer)
}

async fn send_node_has_data(addr: &SocketAddr, processed_data: Vec<u8>, node_id: u128) -> Result<(), NCError> {
    debug!("Connecting to server: {}", addr);
    let mut stream = TcpStream::connect(&addr).await.map_err(|e| NCError::TcpConnect(e))?;
    let (_, writer) = stream.split();
    let mut buf_writer = BufWriter::new(writer);

    debug!("Encoding message HasData");
    let message = nc_encode_data(&NCNodeMessage::HasData((node_id, processed_data)))?;

    debug!("Send message back to server");
    nc_send_message(&mut buf_writer, message).await
}
