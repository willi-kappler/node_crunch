

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

pub use nc_server::{NC_Server, nc_start_server};
pub use nc_node::{NC_Node, nc_start_node};
pub use nc_error::NC_Error;
pub use nc_util::{NC_JobStatus, nc_encode_data, nc_decode_data};
pub use nc_config::{NC_Configuration};
