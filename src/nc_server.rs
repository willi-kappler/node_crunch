use std::sync::{Arc, Mutex};
use std::{thread};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};

use log::{error, info, debug};

use serde::{Serialize, Deserialize};
use std::time::{Instant, Duration};

use crate::{nc_error::{NCError}};
use crate::nc_node::{NCNodeMessage};
use crate::nc_config::{NCConfiguration};
use crate::nc_node_info::{NCNodeInfo, NodeID};
use crate::nc_util::{nc_receive_data, nc_send_data2};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCServerMessage {
    InitialData(NodeID, Option<Vec<u8>>),
    JobStatus(NCJobStatus),
    HeartBeat(bool),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum NCJobStatus {
    Unfinished(Vec<u8>),
    Waiting,
    Finished,
}

type NCNodeInfoList = Arc<Mutex<Vec<NCNodeInfo>>>;

// TODO: Generic trait, U for data in, V for data out
pub trait NCServer {
    fn initial_data(&mut self) -> Result<Option<Vec<u8>>, NCError> {
        Ok(None)
    }
    fn prepare_data_for_node(&mut self, node_id: NodeID) -> Result<NCJobStatus, NCError>;
    fn process_data_from_node(&mut self, node_id: NodeID, data: &Vec<u8>) -> Result<(), NCError>;
    fn heartbeat_timeout(&mut self, node_id: NodeID);
    fn finish_job(&mut self);
}

pub fn nc_start_server<T: 'static + NCServer + Send>(nc_server: T, config: NCConfiguration) -> Result<(), NCError> {
    debug!("Start nc_start_server()");

    let time_start = Instant::now();
    let nc_server = Arc::new(Mutex::new(nc_server));
    let mut node_list = Arc::new(Mutex::new(Vec::<NCNodeInfo>::new()));

    let heartbeat_handle = start_heartbeat_thread(2 * config.heartbeat, node_list.clone(), nc_server.clone());

    start_main_loop(node_list.clone(), nc_server.clone(), config)?;

    {
        // Clear node_list so that the heartbeat_thread can exit
        let mut node_list = node_list.lock()?;
        node_list.clear();
    } // Mutex node_list is unlocked here

    heartbeat_handle.join().map_err(|_| NCError::ThreadJoin)?;

    info!("Call finish_job() for nc_server");

    {
        nc_server.lock()?.finish_job();
    } // Mutex nc_server is unlocked here

    let time_taken = (Instant::now() - time_start).as_secs_f64();

    info!("Job done, exit now");
    info!("Time taken: {} s, {} min, {} h", time_taken, time_taken / 60.0, time_taken / (60.0 * 60.0));

    Ok(())
}

fn start_heartbeat_thread<T: 'static + NCServer + Send>(heartbeat_duration: u64, node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>) -> thread::JoinHandle<()> {
    debug!("Start start_heartbeat_thread(), heartbeat_duration: {}", heartbeat_duration);

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(heartbeat_duration));

            match check_heartbeat(heartbeat_duration, &node_list, &nc_server) {
                Ok(quit) => {
                    if quit { break }
                }
                Err(e) => {
                    error!("Error in check_heartbeat(): {}", e);
                    break;
                }
            }
        }
      debug!("Exit start_heartbeat_thread() main loop");
    })
}

fn check_heartbeat<T: NCServer>(heartbeat_duration: u64, node_list: &NCNodeInfoList, nc_server: &Arc<Mutex<T>>) -> Result<bool, NCError> {
    debug!("Start check_heartbeat(), heartbeat_duration: {}", heartbeat_duration);

    let node_list = node_list.lock()?;

    if node_list.is_empty() { return Ok(true) }

    for node in node_list.iter() {
        if node.heartbeat_invalid(heartbeat_duration) {
            nc_server.lock()?.heartbeat_timeout(node.node_id);
            // Mutex nc_server is unlocked here
        }
    }

    Ok(false)
    // Mutex node_list is unlocked here
}

fn start_main_loop<T: 'static + NCServer + Send>(node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>, config: NCConfiguration) -> Result<(), NCError> {
    debug!("Start start_main_loop()");

    let ip_addr: IpAddr = "0.0.0.0".parse()?; // TODO: Make this configurable
    let socket_addr = SocketAddr::new(ip_addr, config.port);

    let listener = TcpListener::bind(socket_addr)?;
    let quit = Arc::new(Mutex::new(false));
    let mut all_threads = Vec::new();

    // TODO: Maybe use crossbeam::thread::scope

    loop {
        match listener.accept() {
            Ok((stream, addr)) => {
                debug!("Got new connection from node: {}", addr);
                let handle = start_node_thread(stream, quit.clone(), node_list.clone(), nc_server.clone());
                all_threads.push(handle);
            }
            Err(e) => {
                error!("IO error while accepting node connections: {}", e);
                return Err(NCError::IOError(e))
            }
        }

        if *(quit.lock()?) {
            debug!("Quit main loop");
            break
        } // Mutex quit is unlocked here
    }

    debug!("Waiting for all threads to finish...");

    for handle in all_threads {
        if let Err(e) = handle.join() {
            error!("Error in start_main_loop(), could not join thread: {:?}", e);
        }
    }

    Ok(())
}

fn start_node_thread<T: 'static + NCServer + Send>(stream: TcpStream, quit: Arc<Mutex<bool>>, node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>) -> thread::JoinHandle<()> {
    debug!("Start start_node_thread()");

    thread::spawn(move || {
        if let Err(e) = handle_node(stream, node_list, nc_server, quit) {
            error!("Error in start_node_thread(), could not acquire lock: {}", e);
        }
    })
}

fn handle_node<T: NCServer>(mut stream: TcpStream, node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>, quit: Arc<Mutex<bool>>) -> Result<(), NCError> {
    debug!("Start handle_node()");

    let request: NCNodeMessage = nc_receive_data(&mut stream)?;

    match request {
        NCNodeMessage::Register => {
            let node_id = {
                let mut node_list = node_list.lock()?;
                let node_id = get_new_node_id(&node_list);
                node_list.push(NCNodeInfo::new(node_id));
                node_id
            }; // Mutex node_list is unlocked here

            let initial_data = {
                let mut nc_server = nc_server.lock()?;
                nc_server.initial_data()?
            }; // Mutex nc_server is unlocked here

            info!("Registering new node: {}, {}", node_id, stream.peer_addr()?);
            nc_send_data2(&NCServerMessage::InitialData(node_id, initial_data), &mut stream)?;
        }
        NCNodeMessage::NeedsData(node_id) => {
            debug!("Node {} needs data to process", node_id);

            let data_for_node = {
                let mut nc_server = nc_server.lock()?;
                nc_server.prepare_data_for_node(node_id)?
            }; // Mutex nc_server is unlocked here

            match data_for_node {
                NCJobStatus::Unfinished(data) => {
                    debug!("Send data to node");
                    nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Unfinished(data)), &mut stream)?;
                    debug!("Data has been sent to node");
                }
                NCJobStatus::Waiting => {
                    debug!("Waiting for other nodes to finish");
                    nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Waiting), &mut stream)?;
                }
                NCJobStatus::Finished => {
                    debug!("Job is done, will exit handle_node()");
                    nc_send_data2(&NCServerMessage::JobStatus(NCJobStatus::Finished), &mut stream)?;
                    let mut quit = quit.lock()?;
                    *quit = true;
                }
            }
        }
        NCNodeMessage::HeartBeat(node_id) => {
            debug!("Got hearbeat from node: {}", node_id);

            {
                let mut node_list = node_list.lock()?;

                for node in node_list.iter_mut() {
                    if node.node_id == node_id {
                        node.update_heartbeat();
                    }
                }
                // Mutex node_list is unlocked here
            }

            let quit = {
                *(quit.lock()?)
                // Mutex quit is unlocked here
            };

            nc_send_data2(&NCServerMessage::HeartBeat(quit), &mut stream)?;
        }
        NCNodeMessage::HasData(node_id, data) => {
            debug!("Node {} has processed some data and we received the results", node_id);

            let mut nc_server = nc_server.lock()?;
            nc_server.process_data_from_node(node_id, &data)?;
            // Mutex nc_server is unlocked here
        }
        NCNodeMessage::NodeQuit(node_id) => {
            info!("Node {} will quit", node_id);
        }
    }

    Ok(())
}

fn get_new_node_id(all_nodes: &Vec<NCNodeInfo>) -> NodeID {
    let mut new_id: NodeID = NodeID::random();

    'l1: loop {
        for node_info in all_nodes.iter() {
            if node_info.node_id == new_id {
                new_id = NodeID::random();
                continue 'l1
            }
        }

        break
    }

    new_id
}
