//! NodeCrunsh is a crate that allows to distribute computations among multiple nodes.
//! The user of this crate has to implement one trait for the server and one trait for the nodes.
//! Usually the code for the server and the node is inside the same binary and the choice if to
//! run in server mode or node mode is done via configuration or command line argument.
//! See some of the programms in the example folders.

// TODO:
// - Add TLS / encryption / secure connection. Use https ? Use warp (https://github.com/seanmonstar/warp) ?
// - Add authentication ?
// - Add error_counter, if too high exit both node and server
// - Speed up serde: https://github.com/serde-rs/bytes

mod nc_server;
mod nc_node;
mod nc_node_info;
mod nc_error;
mod nc_config;
mod nc_util;
mod array2d;

pub use nc_server::{NCServer, NCJobStatus, nc_start_server};
pub use nc_node::{NCNode, nc_start_node};
pub use nc_node_info::NodeID;
pub use nc_error::NCError;
pub use nc_config::NCConfiguration;
pub use nc_util::{nc_decode_data, nc_decode_data2, nc_encode_data};
pub use array2d::{Array2D, Array2DChunk};
