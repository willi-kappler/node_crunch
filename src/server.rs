use std::net::{TcpListener, SocketAddr};

use failure::Error;

use configuration::{Configuration};

pub fn start_server(configuration: Configuration) -> Result<(), Error> {

    let socket = SocketAddr::new(configuration.server_address.parse()?, configuration.port);
    let listener = TcpListener::bind(socket)?;

    loop {
        match listener.accept() {
            Ok((stream, address)) => {

            }
            Err(e) => {
                warn!("Could not accept client connecton: {}", e)
            }
        }
    }


    Ok(())
}
