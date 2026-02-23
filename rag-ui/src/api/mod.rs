use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
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
pub struct DocumentInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentInfo>,
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

// Chat streaming types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub content: Option<String>,
    pub sources: Option<Vec<SourceInfo>>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub enum ChatEvent {
    Sources(Vec<SourceInfo>),
    Content(String),
    Done,
    Error(String),
}

/// Start a chat stream using fetch with ReadableStream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamRequest {
    pub question: String,
    #[serde(default)]
    pub messages: Vec<ChatStreamMessage>,
}

pub async fn start_chat_stream_with_history<F>(
    question: &str,
    history: Vec<ChatStreamMessage>,
    mut on_event: F,
) -> Result<(), String>
where
    F: FnMut(ChatEvent) + 'static,
{
    use js_sys::{Reflect, Uint8Array};
    use web_sys::{RequestInit, RequestMode, Response};

    let window = web_sys::window().ok_or("No window")?;

    // Create request body with history
    let body = serde_json::to_string(&ChatStreamRequest {
        question: question.to_string(),
        messages: history,
    })
    .map_err(|e| e.to_string())?;

    // Create request options
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body));

    // Create headers
    let headers = web_sys::Headers::new().map_err(|e| format!("{:?}", e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;
    opts.set_headers(&headers);

    let url = format!("{}/chat/stream", API_BASE);
    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("{:?}", e))?;

    let resp_promise = window.fetch_with_request(&request);
    let resp_value = wasm_bindgen_futures::JsFuture::from(resp_promise)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let response: Response = resp_value.dyn_into().map_err(|_| "Invalid response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body = response.body().ok_or("No body")?;
    let reader = body
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|_| "Failed to get reader")?;

    let mut buffer = String::new();

    loop {
        let result = wasm_bindgen_futures::JsFuture::from(reader.read())
            .await
            .map_err(|e| format!("{:?}", e))?;

        let done = Reflect::get(&result, &JsValue::from_str("done"))
            .map_err(|e| format!("{:?}", e))?
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        let value = Reflect::get(&result, &JsValue::from_str("value"))
            .map_err(|e| format!("{:?}", e))?;

        if !value.is_undefined() {
            let array = Uint8Array::new(&value);
            let bytes = array.to_vec();
            let text = String::from_utf8_lossy(&bytes);
            buffer.push_str(&text);

            // Process complete SSE events
            while let Some(event_end) = buffer.find("\n\n") {
                let event_data = buffer[..event_end].to_string();
                buffer = buffer[event_end + 2..].to_string();

                // Parse SSE data line
                for line in event_data.lines() {
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        if let Ok(event) = serde_json::from_str::<ChatStreamEvent>(data) {
                            match event.event_type.as_str() {
                                "sources" => {
                                    if let Some(sources) = event.sources {
                                        on_event(ChatEvent::Sources(sources));
                                    }
                                }
                                "content" => {
                                    if let Some(content) = event.content {
                                        on_event(ChatEvent::Content(content));
                                    }
                                }
                                "done" => {
                                    on_event(ChatEvent::Done);
                                }
                                "error" => {
                                    if let Some(error) = event.error {
                                        on_event(ChatEvent::Error(error));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn start_chat_stream<F>(question: &str, mut on_event: F) -> Result<(), String>
where
    F: FnMut(ChatEvent) + 'static,
{
    use js_sys::{Reflect, Uint8Array};
    use web_sys::{RequestInit, RequestMode, Response};

    let window = web_sys::window().ok_or("No window")?;

    // Create request body
    let body = serde_json::to_string(&ChatStreamRequest {
        question: question.to_string(),
        messages: vec![],
    })
    .map_err(|e| e.to_string())?;

    // Create request options
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body));

    // Create headers
    let headers = web_sys::Headers::new().map_err(|e| format!("{:?}", e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;
    opts.set_headers(&headers);

    let url = format!("{}/chat/stream", API_BASE);
    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("{:?}", e))?;

    let resp_promise = window.fetch_with_request(&request);
    let resp_value = wasm_bindgen_futures::JsFuture::from(resp_promise)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let response: Response = resp_value.dyn_into().map_err(|_| "Invalid response")?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body = response.body().ok_or("No body")?;
    let reader = body
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|_| "Failed to get reader")?;

    let mut buffer = String::new();

    loop {
        let result = wasm_bindgen_futures::JsFuture::from(reader.read())
            .await
            .map_err(|e| format!("{:?}", e))?;

        let done = Reflect::get(&result, &JsValue::from_str("done"))
            .map_err(|e| format!("{:?}", e))?
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        let value = Reflect::get(&result, &JsValue::from_str("value"))
            .map_err(|e| format!("{:?}", e))?;

        if !value.is_undefined() {
            let array = Uint8Array::new(&value);
            let bytes = array.to_vec();
            let text = String::from_utf8_lossy(&bytes);
            buffer.push_str(&text);

            // Process complete SSE events
            while let Some(event_end) = buffer.find("\n\n") {
                let event_data = buffer[..event_end].to_string();
                buffer = buffer[event_end + 2..].to_string();

                // Parse SSE data line
                for line in event_data.lines() {
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        if let Ok(event) = serde_json::from_str::<ChatStreamEvent>(data) {
                            match event.event_type.as_str() {
                                "sources" => {
                                    if let Some(sources) = event.sources {
                                        on_event(ChatEvent::Sources(sources));
                                    }
                                }
                                "content" => {
                                    if let Some(content) = event.content {
                                        on_event(ChatEvent::Content(content));
                                    }
                                }
                                "done" => {
                                    on_event(ChatEvent::Done);
                                }
                                "error" => {
                                    if let Some(error) = event.error {
                                        on_event(ChatEvent::Error(error));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ============= Conversation API =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationInfo {
    pub id: Uuid,
    pub title: String,
    pub user_id: Option<String>,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<MessageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
}

pub async fn list_conversations() -> Result<ConversationListResponse, String> {
    let response = Request::get(&format!("{}/conversations", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<ConversationListResponse>()
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

pub async fn create_conversation(title: Option<String>) -> Result<ConversationInfo, String> {
    let request = CreateConversationRequest {
        title,
        user_id: None,
    };

    let response = Request::post(&format!("{}/conversations", API_BASE))
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<ConversationInfo>()
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

pub async fn get_messages(conversation_id: Uuid) -> Result<MessagesResponse, String> {
    let response = Request::get(&format!("{}/conversations/{}/messages", API_BASE, conversation_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<MessagesResponse>()
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

pub async fn add_message(conversation_id: Uuid, role: &str, content: &str) -> Result<MessageInfo, String> {
    let request = AddMessageRequest {
        role: role.to_string(),
        content: content.to_string(),
    };

    let response = Request::post(&format!("{}/conversations/{}/messages", API_BASE, conversation_id))
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<MessageInfo>()
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

pub async fn delete_conversation(id: Uuid) -> Result<DeleteResponse, String> {
    let response = Request::delete(&format!("{}/conversations/{}", API_BASE, id))
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

// ============= Tools API =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteToolRequest {
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResponse {
    pub success: bool,
    pub output: String,
    pub data: Option<serde_json::Value>,
}

pub async fn list_tools() -> Result<ToolListResponse, String> {
    let response = Request::get(&format!("{}/tools", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<ToolListResponse>()
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

pub async fn execute_tool(name: &str, arguments: serde_json::Value) -> Result<ToolExecutionResponse, String> {
    let request = ExecuteToolRequest { arguments };

    let response = Request::post(&format!("{}/tools/{}/execute", API_BASE, name))
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<ToolExecutionResponse>()
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

// ============= Agents API =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteAgentRequest {
    pub query: String,
    pub agent_name: Option<String>,
    pub conversation_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_type: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResponse {
    pub success: bool,
    pub answer: String,
    pub reasoning: Vec<ReasoningStep>,
    pub sources: Vec<SourceInfo>,
    pub error: Option<String>,
}

pub async fn list_agents() -> Result<AgentListResponse, String> {
    let response = Request::get(&format!("{}/agents", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<AgentListResponse>()
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

pub async fn execute_agent(
    query: &str,
    agent_name: Option<String>,
    conversation_id: Option<Uuid>,
) -> Result<AgentExecutionResponse, String> {
    let request = ExecuteAgentRequest {
        query: query.to_string(),
        agent_name,
        conversation_id,
    };

    let response = Request::post(&format!("{}/agents/execute", API_BASE))
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.ok() {
        response
            .json::<AgentExecutionResponse>()
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
