#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub sender: String,
    pub recipients: Vec<String>,
    pub body: Vec<u8>,
}
