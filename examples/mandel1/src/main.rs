

use structopt::StructOpt;
use log4rs;

mod server;
mod node;

#[derive(StructOpt, Debug)]
#[structopt(name = "simple1")]
pub struct Simple1Opt {
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
    let options = Simple1Opt::from_args();

    if options.server {
        create_logger("nc_server.log");
        // server::run_server(options);
    } else {
        create_logger("nc_node.log");
        // node::run_node(options)
    }
}
