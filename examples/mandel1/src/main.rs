

use std::fs;
use structopt::StructOpt;
use log4rs;
use serde::{Serialize, Deserialize};

use node_crunch::{Array2D};

mod server;
mod node;

#[derive(StructOpt, Debug)]
#[structopt(name = "mandel1")]
pub struct Mandel1Opt {
    #[structopt(short = "s", long = "server")]
    server: bool,

    #[structopt(long = "ip", default_value = "127.0.0.1")]
    ip: String,

    #[structopt(short = "p", long = "port", default_value = "2020")]
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerData {
    chunk_id: u64,
    max_iter: u32,
    x: u64,
    y: u64,
    width: u64,
    height: u64,
    x_step: f64,
    y_step: f64,
    re: f64,
    im: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    chunk_id: u64,
    source: Array2D::<u32>,
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
    let options = Mandel1Opt::from_args();

    if options.server {
        create_logger("nc_server.log");
        server::run_server(options);
    } else {
        let mut postfix: u64 = 1;
        let mut filename = format!("nc_node_{:08}.log", postfix);

        loop {
            if fs::metadata(&filename).is_ok() {
                // Filename for logging already exists, try another one...
                postfix += 1;
                filename = format!("nc_node_{:08}.log", postfix);
            } else {
                break
            }
        }

        create_logger(&filename);
        node::run_node(options)
    }
}
