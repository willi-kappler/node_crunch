//! NodeCrunch is a crate that allows to distribute computations among multiple nodes.
//! The user of this crate has to implement one trait for the server and one trait for the nodes.
//! Usually the code for the server and the node is inside the same binary and the choice if to
//! run in server mode or node mode is done via configuration or command line argument.
//! See some of the programs in the example folders.

// TODO:
// - Add TLS / encryption / secure connection. Use https ? Use warp (https://github.com/seanmonstar/warp) ?
// - Add authentication ?
// - Speed up serde: https://github.com/serde-rs/bytes
// - Maybe use https://docs.rs/orion/0.16.0/orion/auth/struct.SecretKey.html to encrypt messages ?
// - Also have a look into QUIC (https://github.com/cloudflare/quiche)
// - Strobe looks interesting: https://github.com/rozbb/strobe-rs
// - Maybe https://docs.rs/chacha20poly1305/0.9.0/chacha20poly1305/
// - Maybe https://docs.rs/block-modes/0.8.1/block_modes/
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
pub mod array2d;
pub mod nc_communicator;

pub use nc_server::{NCServer, NCJobStatus, NCServerStarter};
pub use nc_node::{NCNode, NCNodeStarter};
pub use nc_node_info::NodeID;
pub use nc_error::NCError;
pub use nc_config::NCConfiguration;
pub use array2d::{Array2D, Array2DChunk, ChunkList, Chunk};
