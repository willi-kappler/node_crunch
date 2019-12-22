

// TODO:
// - Add TLS / encryption / secure connection
// - Add authentication ?
// - Add payload to NodeNeedsData() ?
// - Add error_counter, if too high exit both node and server



mod nc_server;
mod nc_node;
mod nc_error;
mod nc_util;
mod nc_config;

pub use nc_server::{NCServer, nc_start_server};
pub use nc_node::{NCNode, nc_start_node};
pub use nc_error::NCError;
pub use nc_util::{NCJobStatus, nc_encode_data, nc_decode_data};
pub use nc_config::{NCConfiguration};
