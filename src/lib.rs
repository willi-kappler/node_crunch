#[macro_use] extern crate derive_builder;
#[macro_use] extern crate failure;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate rmp_serde;

pub mod error;
pub mod configuration;
pub mod server;
pub mod client;


/*
    Serde overview:
    https://serde.rs/

    MessagePack:
        https://github.com/3Hren/msgpack-rust
        https://docs.rs/rmp-serde/0.13.7/rmp_serde/
        https://crates.io/crates/rmp

    BSON:
        https://github.com/zonyitoo/bson-rs
        https://crates.io/crates/bson


*/
