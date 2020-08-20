use serde::Serialize;

/// A Signal message
#[derive(Serialize)]
pub struct Message {
    /// Address of receiver / sender
    address: String,
    /// Message
    body: String,
}

impl Message {
    pub fn new(sql_parameter: &[rusqlite::types::Value]) -> Self {
        Self {
            address: if let rusqlite::types::Value::Text(x) = sql_parameter[2].to_owned() { x } else { String::from("") },
            body: if let rusqlite::types::Value::Text(x) = sql_parameter[14].to_owned() { x } else { String::from("") },
        }
    }
}
