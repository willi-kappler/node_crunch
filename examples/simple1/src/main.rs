#[macro_use] extern crate log;
extern crate log4rs;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate node_crunch;
extern crate rand;
#[macro_use] extern crate structopt;

mod server;
mod node;

use structopt::StructOpt;

#[derive(Debug, Serialize, Deserialize)]
struct InputData {
    chunck: usize,
    data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutputData {
    chunck: usize,
    data: Vec<u8>,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "simple1")]
struct NCOpt {
    #[structopt(short = "s", long = "server")]
    server: bool,

    #[structopt(long = "ip", default_value = "127.0.0.1")]
    ip: String,

    #[structopt(short = "p", long = "port", default_value = "2020")]
    port: u16,
}

fn create_logger(filename: &str) {
    let file_logger = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("{d} {l} - {m}{n}")))
        .build(filename).unwrap();

    let config = log4rs::config::Config::builder()
        .appender(log4rs::config::Appender::builder().build("file_logger", Box::new(file_logger)))
        .build(log4rs::config::Root::builder().appender("file_logger").build(log::LevelFilter::Debug))
        .unwrap();

    let _log_handle = log4rs::init_config(config).unwrap();
}

fn main() {
    let options = NCOpt::from_args();

    if options.server {
        create_logger("nc_server.log");
        server::run_server(options.port);
    } else {
        create_logger("nc_node.log");
        node::run_node(&options.ip, options.port)
    }

}
