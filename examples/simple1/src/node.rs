// Std modules
use std::{thread, time};


// External crates
use rand::{thread_rng, Rng};


// Internal modules
use node_crunch::{node, configuration};

use InputData;
use OutputData;
use Simple1Opt;

pub fn run_node(options: Simple1Opt) {
    let node_config = configuration::ConfigurationBuilder::default()
        .server_address(options.ip)
        .port(options.port)
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
        let sleep_time = rng.gen_range(1, 20);

        debug!("Node sleep time: {}", sleep_time);

        let duration = time::Duration::from_secs(sleep_time);

        thread::sleep(duration);

        result
    }
}
