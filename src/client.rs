use failure::Error;

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeMessage {
    ReadyForInput,
    OutputData(u8),
}
