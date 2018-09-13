use failure::Error;

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeMessage {
    ReadForInput,
    OutputData,
}
