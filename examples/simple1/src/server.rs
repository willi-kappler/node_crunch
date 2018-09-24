
// External crates
use rand::{thread_rng, Rng};

// Internal modules
use node_crunch::{server, configuration};

use InputData;
use OutputData;
use Simple1Opt;

#[derive(Debug)]
struct TestServer {
    data: Vec<u8>,
    chuncks_processed: Vec<bool>,
    num_of_chuncks: usize,
    chunck_size: usize,
}

pub fn run_server(options: Simple1Opt) {
    let server_config = configuration::ConfigurationBuilder::default()
        .server_address("0.0.0.0")
        .port(options.port)
        .build()
        .unwrap();

    let mut rng = thread_rng();

    let mut test_server = TestServer {
        data: Vec::new(),
        chuncks_processed: Vec::new(),
        num_of_chuncks: 10,
        chunck_size: 20,
    };

    for _ in 0..(test_server.num_of_chuncks * test_server.chunck_size) {
        test_server.data.push(rng.gen_range::<u8>(0, 50));
    }

    for _ in 0..test_server.num_of_chuncks {
        test_server.chuncks_processed.push(false);
    }

    match server::start_server(server_config, test_server) {
        Ok(_) => {
            info!("Server finished");
        }
        Err(e) => {
            error!("An error occured: {:?}", e);
        }
    }
}

impl server::NCServer<InputData, OutputData> for TestServer {
    fn prepare_node_input(&mut self) -> InputData {
        let mut rng = thread_rng();

        if self.is_job_done() {
            debug!("Job is done, send empty data to node");
            InputData{ chunck: 0, data: Vec::new() }
        } else {
            let mut chunck: usize = rng.gen_range(0, 10);

            while self.chuncks_processed[chunck] {
                chunck = rng.gen_range(0, 10);
            }

            debug!("Send chunck {} to node", chunck);
            debug!("Chunck list: {:?}", self.chuncks_processed);

            let start = chunck * self.chunck_size;
            let end = start + self.chunck_size;

            let data = self.data[start..end].to_vec();

            InputData{ chunck, data }
        }
    }

    fn process_node_output(&mut self, result: OutputData) {
        let start = result.chunck * self.chunck_size;;
        let end = start + self.chunck_size;

        for i in start..end {
            self.data[i] = result.data[i - start];
        }

        self.chuncks_processed[result.chunck] = true;
    }

    fn is_job_done(&mut self) -> bool {
        self.chuncks_processed.iter().all(|x| *x)
    }
}
