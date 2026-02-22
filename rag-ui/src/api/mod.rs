use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use web_sys::FormData;

const API_BASE: &str = "http://localhost:8080/api";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub document_id: Uuid,
    pub source_file: String,
    pub excerpt: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub document_id: Uuid,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentListResponse {
    pub documents: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn query_documents(question: &str) -> Result<QueryResponse, String> {
    let request = QueryRequest {
        question: question.to_string(),
    };

    let response = Request::post(&format!("{}/query", API_BASE))
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<QueryResponse>()
            .await
            .map_err(|e| e.to_string())
    } else {
        let error = response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(error)
    }
}

pub async fn list_documents() -> Result<DocumentListResponse, String> {
    let response = Request::get(&format!("{}/documents/documents", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<DocumentListResponse>()
            .await
            .map_err(|e| e.to_string())
    } else {
        let error = response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(error)
    }
}

pub async fn upload_document(form_data: FormData) -> Result<UploadResponse, String> {
    let response = Request::post(&format!("{}/documents/documents", API_BASE))
        .body(form_data)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<UploadResponse>()
            .await
            .map_err(|e| e.to_string())
    } else {
        let error = response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(error)
    }
}

pub async fn delete_document(id: Uuid) -> Result<DeleteResponse, String> {
    let response = Request::delete(&format!("{}/documents/documents/{}", API_BASE, id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<DeleteResponse>()
            .await
            .map_err(|e| e.to_string())
    } else {
        let error = response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(error)
    }
}
