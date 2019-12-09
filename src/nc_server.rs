use tokio::net::TcpListener;
use tokio::io::{BufReader, BufWriter, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};

use crate::error::{NCError};
use crate::nc_node::{NodeMessage};

pub enum ServerMessage {
    ServerHasData(Vec<u8>),
    ServerFinished,
}

pub trait NC_Server {
    fn finished(&self) -> bool;
    fn prepare_data_for_node(&mut self) -> Vec<u8>;
    fn process_data_from_node(&mut self, data: Vec<u8>);
}

async fn start_server<T: NC_Server>(server: T) -> Result<(), NCError> {

    let addr = "127.0.0.1:9000".to_string();
    let mut socket = TcpListener::bind(&addr).await?;
    debug!("Listening on: {}", addr);

    let mut quit = false;

    while !quit {
        if let Ok((mut stream, peer)) = socket.accept().await {
            debug!("Connection from: {}", peer.to_string());

            tokio::spawn(async move {
                let (reader, mut writer) = stream.split();
                let mut buf_reader = BufReader::new(reader);
                
                let message_length = reader.read_u64().await?
                let mut buffer = vec![0; message_length];
                let num_of_bytes_read = reader.read(&mut buffer[..]).await?;

                debug!("Message length: {}, number of bytes read: {}", message_length, num_of_bytes_read);

                match decode(buffer)? {
                    NodeMessage::NodeNeedsData => {

                    }
                    NodeMessage::NodeHasData(new_data) => {

                    }
                }
            });
        }
    }

    Ok(())
}

fn decode(buffer: Vec<u8>) -> Result<NodeMessage, NCError> {
    Ok(NodeMessage::NodeNeedsData)
}
