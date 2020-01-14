

// TODO:
// - Add heart beat from node
// - Add TLS / encryption / secure connection
// - Add authentication ?
// - Add payload to NodeNeedsData() ?
// - Add error_counter, if too high exit both node and server
// - Speed up serde: https://github.com/serde-rs/bytes


mod nc_server;
mod nc_node;
mod nc_error;
mod nc_util;
mod nc_config;
