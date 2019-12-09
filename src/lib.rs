use tokio::net::TcpListener;
use tokio::io::{BufReader, AsyncBufReadExt, AsyncWriteExt};

use log::{info, error, debug};

async fn start_server() -> Result<(), ()> {

    let addr = "127.0.0.1:9000".to_string();
    let mut socket = TcpListener::bind(&addr).await.unwrap();
    debug!("Listening on: {}", addr);

    let mut quit = false;

    while !quit {
        if let Ok((mut stream, peer)) = socket.accept().await {
            debug!("Connection from: {}", peer.to_string());

            tokio::spawn(async move {
                let (reader, mut writer) = stream.split();
                let mut buf_reader = BufReader::new(reader);
                let mut buf = Vec::<u8>::new();
            });
        }
    }

    Ok(())
}
