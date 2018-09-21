

#[derive(Default, Builder, Debug, Clone, Eq, PartialEq)]
#[builder(setter(into))]
pub struct Configuration {
    pub server_address: String,
    pub port: u16,
    pub timeout: u64,
}
