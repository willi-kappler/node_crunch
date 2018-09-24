#[macro_use] extern crate log;
extern crate log4rs;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate node_crunch;
extern crate rand;

// Std modules
use std::{thread, time};

// External crates
use rand::{thread_rng, Rng};

// Internal modules
use node_crunch::{node, configuration};

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

#[derive(Debug)]
struct TestNode {
}

impl node::NCNode<InputData, OutputData> for TestNode {
    fn process_new_data_from_server(&mut self, input: InputData) -> OutputData {
        debug!("Processing chunck: {}", input.chunck);

        let mut result = OutputData { chunck: input.chunck, data: Vec::new() };

        for value in input.data {
            result.data.push(value + 100);
        }

        let mut rng = thread_rng();
        let sleep_time = rng.gen_range(2, 5);

        debug!("Node sleep time: {}", sleep_time);

        let duration = time::Duration::from_secs(sleep_time);

        thread::sleep(duration);

        result
    }
}

fn main() {
    let file_logger = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("{d} {l} - {m}{n}")))
        .build("node.log").unwrap();

    let config = log4rs::config::Config::builder()
        .appender(log4rs::config::Appender::builder().build("file_logger", Box::new(file_logger)))
        .build(log4rs::config::Root::builder().appender("file_logger").build(log::LevelFilter::Debug))
        .unwrap();

    let _log_handle = log4rs::init_config(config).unwrap();

    let node_config = configuration::ConfigurationBuilder::default()
        .server_address("127.0.0.1")
        .port(2020u16)
        .timeout(10u64)
        .build()
        .unwrap();

    let test_node = TestNode{};

    match node::start_node(node_config, test_node) {
        Ok(_) => {
            info!("Node finished");
        }
        Err(e) => {
            error!("An error occured: {:?}", e);
        }
    }
}
