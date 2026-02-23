use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration error: {0}")]
    LoadError(#[from] config::ConfigError),

    #[error("Missing required configuration: {0}")]
    MissingValue(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub openai: OpenAISettings,
    pub rag: RagSettings,
    #[serde(default)]
    pub security: SecuritySettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecuritySettings {
    /// API keys that are allowed to access the API (comma-separated in env var)
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// Whether to require API key authentication
    #[serde(default = "default_require_auth")]
    pub require_auth: bool,
    /// Rate limit: requests per minute per IP
    #[serde(default = "default_rate_limit_rpm")]
    pub rate_limit_rpm: u32,
    /// Rate limit: requests per minute for expensive operations (chat, agents)
    #[serde(default = "default_expensive_rate_limit_rpm")]
    pub expensive_rate_limit_rpm: u32,
    /// Maximum query length in characters
    #[serde(default = "default_max_query_length")]
    pub max_query_length: usize,
    /// Maximum message content length
    #[serde(default = "default_max_message_length")]
    pub max_message_length: usize,
    /// Maximum tokens per response (cost control)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Allowed CORS origins (comma-separated, empty = allow all)
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Enable prompt injection detection
    #[serde(default = "default_true")]
    pub prompt_injection_detection: bool,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            api_keys: Vec::new(),
            require_auth: default_require_auth(),
            rate_limit_rpm: default_rate_limit_rpm(),
            expensive_rate_limit_rpm: default_expensive_rate_limit_rpm(),
            max_query_length: default_max_query_length(),
            max_message_length: default_max_message_length(),
            max_tokens: default_max_tokens(),
            allowed_origins: Vec::new(),
            prompt_injection_detection: true,
        }
    }
}

fn default_require_auth() -> bool {
    false // Default to false for development, set to true in production
}

fn default_rate_limit_rpm() -> u32 {
    60 // 60 requests per minute
}

fn default_expensive_rate_limit_rpm() -> u32 {
    20 // 20 expensive requests per minute
}

fn default_max_query_length() -> usize {
    10000 // 10k characters max
}

fn default_max_message_length() -> usize {
    50000 // 50k characters max for message content
}

fn default_max_tokens() -> u32 {
    4096 // Max tokens per response
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default = "default_database_path")]
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAISettings {
    pub api_key: String,
    #[serde(default = "default_api_base")]
    pub api_base: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    #[serde(default = "default_chat_model")]
    pub chat_model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RagSettings {
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    /// Minimum relevance score (0.0-1.0) for a document to be considered relevant
    /// Questions with all results below this threshold will be rejected as out-of-context
    #[serde(default = "default_min_relevance_score")]
    pub min_relevance_score: f32,
}

fn default_min_relevance_score() -> f32 {
    0.10 // Reject if best match is below 10% relevance (very permissive)
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_database_path() -> String {
    "./data/rag.db".to_string()
}

fn default_api_base() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_chat_model() -> String {
    "gpt-4o".to_string()
}

fn default_chunk_size() -> usize {
    1000
}

fn default_chunk_overlap() -> usize {
    200
}

fn default_top_k() -> usize {
    5
}

fn default_system_prompt() -> String {
    r#"You answer questions using ONLY the provided context. Be direct and concise.

RULES:
1. Give the answer directly - NO preamble phrases like "Based on the context", "According to the document", "From the information provided", etc.
2. Just state the facts. Example: Q: "What is his email?" A: "maulanasdqn@gmail.com"
3. If you cannot answer from the context, say: "I don't have that information."
4. Keep answers to 1-3 sentences max.
5. Never make up information."#.to_string()
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        config.try_deserialize().map_err(ConfigError::from)
    }
}
