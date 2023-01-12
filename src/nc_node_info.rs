
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NCNodeInfo {
    id: String,
}

impl NCNodeInfo {
    pub(crate) fn empty() -> Self {
        Self {
            id: "".to_string(),
        }
    }
}
