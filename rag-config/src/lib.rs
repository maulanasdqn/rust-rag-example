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
    "Answer questions using the provided context. Be brief and direct - give only the essential information in 1-3 sentences. No unnecessary explanations."
        .to_string()
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        config.try_deserialize().map_err(ConfigError::from)
    }
}
