use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UploadDocumentRequest {
    pub filename: String,
}
