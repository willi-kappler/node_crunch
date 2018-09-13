// Std modules
use std::net::{TcpListener, SocketAddr, TcpStream};

// External crates
use failure::Error;

// Internal modules
use configuration::{Configuration};
use client::{NodeMessage};

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    ProcessData,
    JobFinished,
}

pub trait NCServer {
    fn node_ready_for_input();

    fn node_output_data();

    fn is_job_done() -> bool;
}


pub fn start_server(configuration: Configuration) -> Result<(), Error> {

    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    let listener = TcpListener::bind(socket)?;

    loop {
        match listener.accept() {
            Ok((stream, address)) => {
                info!("Connection from node: {}", address);
                handle_client(stream);
            }
            Err(e) => {
                warn!("Could not accept node connecton: {:?}", e)
            }
        }
    }


    Ok(())
}

fn handle_client(stream: TcpStream) {

}
