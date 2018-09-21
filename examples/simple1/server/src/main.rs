#[macro_use] extern crate log;
extern crate log4rs;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate node_crunch;
extern crate rand;

// External crates
use rand::{thread_rng, Rng};

// Internal crates
use node_crunch::{server, configuration};

#[derive(Debug, Serialize)]
struct InputData {
    chunck: usize,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct OutputData {
    chunck: usize,
    data: Vec<u8>,
}

#[derive(Debug)]
struct TestServer {
    data: Vec<u8>,
    chuncks_processed: Vec<bool>,
    num_of_chuncks: usize,
    chunck_size: usize,
}

impl server::NCServer<InputData, OutputData> for TestServer {
    fn prepare_node_input(&mut self) -> InputData {
        let mut rng = thread_rng();

        if self.is_job_done() {
            InputData{ chunck: 0, data: Vec::new() }
        } else {
            let mut chunck: usize = rng.gen_range(0, 10);

            while self.chuncks_processed[chunck] {
                chunck = rng.gen_range(0, 10);
            }

            debug!("Send chunck {} to node", chunck);

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

fn main() {
    let file_logger = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("{d} {l} - {m}{n}")))
        .build("server.log").unwrap();

    let config = log4rs::config::Config::builder()
        .appender(log4rs::config::Appender::builder().build("file_logger", Box::new(file_logger)))
        .build(log4rs::config::Root::builder().appender("file_logger").build(log::LevelFilter::Debug))
        .unwrap();

    let _log_handle = log4rs::init_config(config).unwrap();

    let server_config = configuration::ConfigurationBuilder::default()
        .server_address("0.0.0.0")
        .port(2020u16)
        .timeout(10u64)
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
