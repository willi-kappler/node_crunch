//! NodeCrunch is a crate that allows to distribute computations among multiple nodes.
//! The user of this crate has to implement one trait for the server and one trait for the nodes.
//! Usually the code for the server and the node is inside the same binary and the choice if to
//! run in server mode or node mode is done via configuration or command line argument.
//! See some of the programs in the example folders.

// TODO:
// - Add TLS / encryption / secure connection. Use https ? Use warp (https://github.com/seanmonstar/warp) ?
// - Add authentication ?
// - Speed up serde: https://github.com/serde-rs/bytes
// - Use generic data instead of Vec<u8> ?
// - Maybe use https://docs.rs/orion/0.16.0/orion/auth/struct.SecretKey.html to encrypt messages ?
// - Maybe use the quick protocoll: https://github.com/cloudflare/quiche
//
// Examples to add:
// - Add an example with https://github.com/wahn/rs_pbrt, https://github.com/TwinkleBear/tray_rust
// - Distributed machine learning ?
// - Distributed black hole simulation ?
// - Distributed crypto mining ?
//
// Own projects that will use node_crunch:
// - Darwin: https://github.com/willi-kappler/darwin-rs
// - GRONN: https://github.com/willi-kappler/gronn

pub mod nc_server;
pub mod nc_node;
pub mod nc_node_info;
pub mod nc_error;
pub mod nc_config;
pub mod nc_util;
pub mod array2d;

pub use nc_server::{NCServer, NCJobStatus, NCServerStarter};
pub use nc_node::{NCNode, NCNodeStarter};
pub use nc_node_info::NodeID;
pub use nc_error::NCError;
pub use nc_config::NCConfiguration;
pub use nc_util::{nc_decode_data, nc_decode_data2, nc_encode_data};
pub use array2d::{Array2D, Array2DChunk, ChunkList, Chunk};
