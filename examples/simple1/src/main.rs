#[macro_use] extern crate log;
extern crate log4rs;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate node_crunch;
extern crate rand;

#[derive(Debug, Deserialize)]
struct InputData {
    chunck: usize,
    data: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct OutputData {
    chunck: usize,
    data: Vec<u8>,
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

}
