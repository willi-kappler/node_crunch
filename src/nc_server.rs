use std::sync::{Arc, Mutex};

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{BufReader, BufWriter, AsyncReadExt, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};

use crate::error::{NCError};
use crate::nc_node::{NodeMessage};

#[derive(Debug, Clone)]
pub enum ServerMessage {
    ServerHasData(Vec<u8>),
    ServerFinished,
}

pub trait NC_Server {
    fn finished(&self) -> bool;
    fn prepare_data_for_node(&mut self) -> Vec<u8>;
    fn process_data_from_node(&mut self, data: &Vec<u8>);
}

async fn start_server<T: 'static + NC_Server + Send>(nc_server: T) -> Result<(), NCError> {
    let addr = "127.0.0.1:9000".to_string();
    let mut socket = TcpListener::bind(&addr).await.map_err(|e| NCError::TcpBind(e))?;

    debug!("Listening on: {}", addr);

    let quit = Arc::new(Mutex::new(false));
    let nc_server = Arc::new(Mutex::new(nc_server));

    while !(*quit.lock().map_err(|_| NCError::QuitLock)?) {
        let (stream, node) = socket.accept().await.map_err(|e| NCError::SocketAccept(e))?;
        let nc_server = nc_server.clone();
        let quit = quit.clone();

        debug!("Connection from: {}", node.to_string());

        tokio::spawn(async move {
            match handle_node(nc_server, stream, quit).await {
                Ok(_) => debug!("handle node finished"),
                Err(e) => debug!("handle node returned an error: {}", e),
            }
        });
    }

    Ok(())
}

async fn handle_node<T: NC_Server>(nc_server: Arc<Mutex<T>>, mut stream: TcpStream, quit: Arc<Mutex<bool>>) -> Result<(), NCError> {
    let (reader, writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut buf_writer = BufWriter::new(writer);
    
    let message_length: u64 = buf_reader.read_u64().await.map_err(|e| NCError::ReadU64(e))?;
    let mut buffer = vec![0; message_length as usize];
    let num_of_bytes_read: usize = buf_reader.read(&mut buffer[..]).await.map_err(|e| NCError::ReadBuffer(e))?;

    debug!("handle_node: Message length: {}, number of bytes read: {}", message_length, num_of_bytes_read);

    match decode(buffer)? {
        NodeMessage::NodeNeedsData => {
            let quit = *quit.lock().map_err(|_| NCError::QuitLock)?;
            if quit {
                let message = encode(ServerMessage::ServerFinished)?;

                buf_writer.write_u64(message.len() as u64).await.map_err(|e| NCError::WriteU64(e))?;
                buf_writer.write(&message).await.map_err(|e| NCError::WriteBuffer(e))?;
            } else {
                let new_data = {
                    let mut nc_server = nc_server.lock().map_err(|_| NCError::ServerLock)?;
                    nc_server.prepare_data_for_node() // TODO: this may take a lot of time
                };

                let message = encode(ServerMessage::ServerHasData(new_data))?;
                let message_length = message.len() as u64;
    
                buf_writer.write_u64(message_length).await.map_err(|e| NCError::WriteU64(e))?;
                buf_writer.write(&message).await.map_err(|e| NCError::WriteBuffer(e))?;
    
                debug!("New data sent to node, message_length: {}", message_length);
            }
        }
        NodeMessage::NodeHasData(new_data) => {
            let finished = {
                let mut nc_server = nc_server.lock().map_err(|_| NCError::ServerLock)?;
                nc_server.process_data_from_node(&new_data);  // TODO: this may take a lot of time
                nc_server.finished()
            };

            debug!("New processed data received from node");

            if finished {
                {
                    let mut quit = quit.lock().map_err(|_| NCError::QuitLock)?;
                    *quit = true;
                }

                let message = encode(ServerMessage::ServerFinished)?;

                buf_writer.write_u64(message.len() as u64).await.map_err(|e| NCError::WriteU64(e))?;
                buf_writer.write(&message).await.map_err(|e| NCError::WriteBuffer(e))?;

                debug!("Job is finished!");
            }
        }
    }

    Ok(())
}

fn decode(buffer: Vec<u8>) -> Result<NodeMessage, NCError> {
    Ok(NodeMessage::NodeNeedsData)
}

fn encode(message: ServerMessage) -> Result<Vec<u8>, NCError> {
    let mut data: Vec<u8> = Vec::new();
    Ok(data)
}
