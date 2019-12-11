use std::sync::{Arc, Mutex};
use std::error;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{BufReader, BufWriter, AsyncReadExt, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use bincode::{deserialize, serialize};

use crate::nc_error::{NC_Error};
use crate::nc_node::{NC_NodeMessage};
use crate::nc_util::{nc_send_message, nc_receive_message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NC_ServerMessage {
    ServerHasData(Vec<u8>),
    ServerFinished,
}

pub trait NC_Server {
    fn finished(&self) -> bool;
    fn prepare_data_for_node(&mut self) -> Result<Vec<u8>, u8>;
    fn process_data_from_node(&mut self, data: &Vec<u8>) -> Result<(), u8>;
}

pub async fn start_server<T: 'static + NC_Server + Send>(nc_server: T) -> Result<(), NC_Error> {
    let addr = "0.0.0.0:9000".to_string(); // TODO: read from config file
    let mut socket = TcpListener::bind(&addr).await.map_err(|e| NC_Error::TcpBind(e))?;

    debug!("Listening on: {}", addr);

    let quit = Arc::new(Mutex::new(false));
    let nc_server = Arc::new(Mutex::new(nc_server));

    while !(*quit.lock().map_err(|_| NC_Error::QuitLock)?) {
        let (stream, node) = socket.accept().await.map_err(|e| NC_Error::SocketAccept(e))?;
        let nc_server = nc_server.clone();
        let quit = quit.clone();

        debug!("Connection from: {}", node.to_string());

        tokio::spawn(async move {
            match handle_node(nc_server, stream, quit).await {
                Ok(_) => debug!("handle node finished"),
                Err(e) => error!("handle node returned an error: {}", e),
            }
        });
    }

    Ok(())
}

async fn handle_node<T: NC_Server>(nc_server: Arc<Mutex<T>>, mut stream: TcpStream, quit: Arc<Mutex<bool>>) -> Result<(), NC_Error> {
    let (reader, writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut buf_writer = BufWriter::new(writer);
    
    let (num_of_bytes_read, buffer) = nc_receive_message(&mut buf_reader).await?;

    debug!("handle_node: number of bytes read: {}", num_of_bytes_read);

    match nc_decode(buffer)? {
        NC_NodeMessage::NodeNeedsData => {
            let quit = *quit.lock().map_err(|_| NC_Error::QuitLock)?;
            if quit {
                let message = nc_encode(NC_ServerMessage::ServerFinished)?;

                nc_send_message(&mut buf_writer, message).await?;

                debug!("No more data for node, server has finished");
            } else {
                let new_data = {
                    let mut nc_server = nc_server.lock().map_err(|_| NC_Error::ServerLock)?;
                    nc_server.prepare_data_for_node().map_err(|e| NC_Error::ServerPrepare(e))? // TODO: this may take a lot of time
                }; // Mutex for nc_server needs to be dropped here

                let message: Vec<u8> = nc_encode(NC_ServerMessage::ServerHasData(new_data))?;
                let message_length = message.len() as u64;

                nc_send_message(&mut buf_writer, message).await?;
    
                debug!("New data sent to node, message_length: {}", message_length);
            }
        }
        NC_NodeMessage::NodeHasData(new_data) => {
            let finished = {
                let mut nc_server = nc_server.lock().map_err(|_| NC_Error::ServerLock)?;
                nc_server.process_data_from_node(&new_data).map_err(|e| NC_Error::ServerProcess(e));  // TODO: this may take a lot of time
                nc_server.finished()
            }; // Mutex for nc_server needs to be dropped here

            debug!("New processed data received from node");

            if finished {
                {
                    let mut quit = quit.lock().map_err(|_| NC_Error::QuitLock)?;
                    *quit = true;
                } // Mutex for quit needs to be dropped here

                let message: Vec<u8> = nc_encode(NC_ServerMessage::ServerFinished)?;

                nc_send_message(&mut buf_writer, message).await?;

                debug!("Job is finished!");
            }
        }
    }

    Ok(())
}

fn nc_encode(message: NC_ServerMessage) -> Result<Vec<u8>, NC_Error> {
    serialize(&message).map_err(|e| NC_Error::Serialize(e))
}

fn nc_decode(buffer: Vec<u8>) -> Result<NC_NodeMessage, NC_Error> {
    deserialize(&buffer).map_err(|e| NC_Error::Deserialize(e))
}
