use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub question: String,
}
