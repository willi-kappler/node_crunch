use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::{thread, time};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::io::{ErrorKind};

use log::{error, debug};

use serde::{Serialize, Deserialize};

use crate::{nc_error::{NCError}};
use crate::nc_node::{NCNodeMessage};
use crate::nc_config::{NCConfiguration};
use crate::nc_node_info::{NCNodeInfo, NodeID};
use crate::nc_util::{nc_receive_data, nc_send_data2};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum NCServerMessage {
    InitialData(NodeID, Option<Vec<u8>>),
    JobStatus(NCJobStatus),
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
    let nc_server = Arc::new(Mutex::new(nc_server));
    let node_list = Arc::new(Mutex::new(Vec::<NCNodeInfo>::new()));

    let heartbeat_handle = start_heartbeat_thread(2 * config.heartbeat, node_list.clone(), nc_server.clone());

    start_main_loop(node_list, nc_server.clone(), config)?;

    heartbeat_handle.join().map_err(|_| NCError::ThreadJoin)?;

    debug!("Call finish_job() for nc_server");

    nc_server.lock()?.finish_job();

    debug!("Job done, exit now");

    Ok(())
    // Mutex nc_server is unlocked here
}

fn start_heartbeat_thread<T: 'static + NCServer + Send>(heartbeat_duration: u64, node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>) -> thread::JoinHandle<()> {
    debug!("Start start_heartbeat_thread(), heartbeat_duration: {}", heartbeat_duration);

    thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::from_secs(heartbeat_duration));

            if let Err(e) = check_heartbeat(heartbeat_duration, &node_list, &nc_server) {
                error!("Error in check_heartbeat(), could not acquire mutex: {}", e);
            }
        }
    })
}

fn check_heartbeat<T: NCServer>(heartbeat_duration: u64, node_list: &NCNodeInfoList, nc_server: &Arc<Mutex<T>>) -> Result<(), NCError> {
    debug!("Start check_heartbeat(), heartbeat_duration: {}", heartbeat_duration);

    let node_list = node_list.lock()?;
    for node in node_list.iter() {
        if node.heartbeat_invalid(heartbeat_duration) {
            nc_server.lock()?.heartbeat_timeout(node.node_id);
            // Mutex nc_server is unlocked here
        }
    }

    Ok(())
    // Mutex node_list is unlocked here
}

fn start_main_loop<T: 'static + NCServer + Send>(node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>, config: NCConfiguration) -> Result<(), NCError> {
    debug!("Start start_main_loop()");

    let ip_addr: IpAddr = "0.0.0.0".parse()?; // TODO: Make this configurable
    let socket_addr = SocketAddr::new(ip_addr, config.port);

    let listener = TcpListener::bind(socket_addr)?;
    let quit = Arc::new(Mutex::new(false));
    let mut all_threads = Vec::new();
    let mut timeout_start = Instant::now();

    listener.set_nonblocking(true)?;

    // TODO: Maybe use crossbeam::thread::scope

    loop {
        match listener.accept() {
            Ok((stream, addr)) => {
                debug!("Got new connection from node: {}", addr);
                let handle = start_node_thread(stream, quit.clone(), node_list.clone(), nc_server.clone());
                all_threads.push(handle);
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                let diff = Instant::now() - timeout_start;
                if diff.as_secs() < 60 { // TODO: Make this configurable
                    thread::sleep(Duration::from_secs(2));
                    continue
                }

                debug!("Accept timeout, time to check if job is done");
                timeout_start = Instant::now();
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
        match handle_node(stream, node_list, nc_server) {
            Ok(result) => {
                match quit.lock() {
                    Ok(mut quit) => {
                        *quit = result;
                    }
                    Err(e) => {
                        error!("Error in start_node_thread(), could not acquire lock: {}", e);
                    }
                }
                // Mutex quit is unlocked here
            }
            Err(e) => {
                error!("Error in handle_node(): {} ", e);
            }
        }
    })
}

fn handle_node<T: NCServer>(mut stream: TcpStream, node_list: NCNodeInfoList, nc_server: Arc<Mutex<T>>) -> Result<bool, NCError> {
    debug!("Start handle_node()");

    let request: NCNodeMessage =  nc_receive_data(&mut stream)?;

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

            debug!("Registering new node: {}, {}", node_id, stream.peer_addr()?);
            nc_send_data2(&NCServerMessage::InitialData(node_id, initial_data), &mut stream)?;
        }
        NCNodeMessage::NeedsData(node_id) => {
            debug!("Node {} needs data to process", node_id);

            let data_for_node = {
                let mut nc_server = nc_server.lock()?;
                nc_server.prepare_data_for_node(node_id)?
            }; // Mutex nc_server is unlocked here

            let quit = data_for_node == NCJobStatus::Finished;

            nc_send_data2(&NCServerMessage::JobStatus(data_for_node), &mut stream)?;

            return Ok(quit)
        }
        NCNodeMessage::HasData(node_id, data) => {
            debug!("Node {} has processed some data and we received the results", node_id);

            let mut nc_server = nc_server.lock()?;
            nc_server.process_data_from_node(node_id, &data).map(|_| false)?;
            // Mutex nc_server is unlocked here
        }
        NCNodeMessage::HeartBeat(node_id) => {
            debug!("Got hearbeat from node: {}", node_id);

            let mut node_list = node_list.lock()?;

            for node in node_list.iter_mut() {
                if node.node_id == node_id {
                    node.update_heartbeat();
                }
            }
            // Mutex node_list is unlocked here
        }
    }

    Ok(false)
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
