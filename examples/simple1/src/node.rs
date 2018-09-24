// Std modules
use std::{thread, time};


// External crates
use rand::{thread_rng, Rng};


// Internal modules
use node_crunch::{node, configuration};
use InputData;
use OutputData;

pub fn run_node(ip: &str, port: u16) {
    let node_config = configuration::ConfigurationBuilder::default()
        .server_address(ip)
        .port(port)
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
        let sleep_time = rng.gen_range(10, 20);

        debug!("Node sleep time: {}", sleep_time);

        let duration = time::Duration::from_secs(sleep_time);

        thread::sleep(duration);

        result
    }
}
