//! Telegram bot integration for the RAG application
//!
//! This module provides a Telegram bot interface that allows users to interact
//! with the RAG system through Telegram messages.

use std::collections::HashMap;
use std::sync::Arc;

use rag_config::TelegramSettings;
use rag_memory::MessageRole;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::use_cases::AppState;

/// Alias for rag_memory::Message to avoid conflict with teloxide::types::Message
type RagMessage = rag_memory::Message;

/// User session tracking (Telegram user_id -> conversation_id)
type UserSessions = Arc<RwLock<HashMap<u64, Uuid>>>;

/// Telegram bot commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Start the bot and see welcome message")]
    Start,
    #[command(description = "Show available commands")]
    Help,
    #[command(description = "Start a new conversation")]
    New,
    #[command(description = "Show conversation history summary")]
    History,
    #[command(description = "List uploaded documents")]
    Docs,
    #[command(description = "Clear conversation history")]
    Clear,
}

/// Start the Telegram bot
pub async fn run_telegram_bot(settings: TelegramSettings, app_state: AppState) {
    if !settings.enabled {
        info!("Telegram bot is disabled");
        return;
    }

    if settings.bot_token.is_empty() {
        warn!("Telegram bot token is empty, skipping bot initialization");
        return;
    }

    info!("Starting Telegram bot...");

    let bot = Bot::new(&settings.bot_token);
    let user_sessions: UserSessions = Arc::new(RwLock::new(HashMap::new()));
    let max_message_length = settings.max_message_length;

    // Create handler with dependencies
    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(
            dptree::filter(|msg: teloxide::types::Message| msg.document().is_some())
                .endpoint(handle_document),
        )
        .branch(dptree::endpoint(handle_text_message));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            app_state,
            user_sessions,
            max_message_length
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Handle bot commands
async fn handle_command(
    bot: Bot,
    msg: teloxide::types::Message,
    cmd: Command,
    app_state: AppState,
    user_sessions: UserSessions,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);

    match cmd {
        Command::Start => {
            let welcome = r#"Welcome to the RAG Assistant Bot!

I can answer questions based on uploaded documents. Just send me a message and I'll search the knowledge base.

Commands:
/new - Start a fresh conversation
/docs - List available documents
/history - View conversation history
/clear - Clear your conversation
/help - Show this help message

Send me any question to get started!"#;

            bot.send_message(chat_id, welcome).await?;
        }

        Command::Help => {
            bot.send_message(chat_id, Command::descriptions().to_string())
                .await?;
        }

        Command::New => {
            // Create a new conversation for this user
            let conv = match app_state
                .conversation_store
                .create_conversation(Some(&user_id.to_string()), Some("Telegram Chat"))
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create conversation: {}", e);
                    bot.send_message(chat_id, "Failed to create new conversation. Please try again.")
                        .await?;
                    return Ok(());
                }
            };

            // Update session
            user_sessions.write().await.insert(user_id, conv.id);

            bot.send_message(chat_id, "Started a new conversation! Your previous chat history has been cleared.")
                .await?;
        }

        Command::History => {
            let sessions = user_sessions.read().await;
            if let Some(&conv_id) = sessions.get(&user_id) {
                match app_state
                    .conversation_store
                    .get_recent_messages(conv_id, 10)
                    .await
                {
                    Ok(messages) => {
                        if messages.is_empty() {
                            bot.send_message(chat_id, "No conversation history yet.")
                                .await?;
                        } else {
                            let mut history = String::from("Recent conversation:\n\n");
                            for m in messages.iter().take(10) {
                                let role = match m.role {
                                    MessageRole::User => "You",
                                    MessageRole::Assistant => "Bot",
                                    _ => "System",
                                };
                                let content: String = m.content.chars().take(100).collect();
                                let ellipsis = if m.content.len() > 100 { "..." } else { "" };
                                history.push_str(&format!("{}: {}{}\n\n", role, content, ellipsis));
                            }
                            bot.send_message(chat_id, history).await?;
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch history: {}", e);
                        bot.send_message(chat_id, "Failed to fetch conversation history.")
                            .await?;
                    }
                }
            } else {
                bot.send_message(chat_id, "No active conversation. Send a message to start one!")
                    .await?;
            }
        }

        Command::Docs => {
            match app_state.list_documents.execute().await {
                Ok(doc_ids) => {
                    if doc_ids.is_empty() {
                        bot.send_message(chat_id, "No documents uploaded yet.")
                            .await?;
                    } else {
                        let mut doc_list = String::from("Available documents:\n\n");
                        for id in doc_ids.iter().take(20) {
                            if let Ok(Some(info)) = app_state.query_documents.get_document_info(*id).await {
                                doc_list.push_str(&format!("- {}\n", info.source_file));
                            }
                        }
                        if doc_ids.len() > 20 {
                            doc_list.push_str(&format!("\n...and {} more", doc_ids.len() - 20));
                        }
                        bot.send_message(chat_id, doc_list).await?;
                    }
                }
                Err(e) => {
                    error!("Failed to list documents: {}", e);
                    bot.send_message(chat_id, "Failed to fetch document list.")
                        .await?;
                }
            }
        }

        Command::Clear => {
            let mut sessions = user_sessions.write().await;
            if let Some(&conv_id) = sessions.get(&user_id) {
                if let Err(e) = app_state.conversation_store.delete_conversation(conv_id).await {
                    error!("Failed to delete conversation: {}", e);
                }
            }
            sessions.remove(&user_id);
            bot.send_message(chat_id, "Conversation cleared! Send a message to start fresh.")
                .await?;
        }
    }

    Ok(())
}

/// Handle document uploads
async fn handle_document(
    bot: Bot,
    msg: teloxide::types::Message,
    app_state: AppState,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;

    let document = match msg.document() {
        Some(doc) => doc,
        None => {
            bot.send_message(chat_id, "No document found in message.")
                .await?;
            return Ok(());
        }
    };

    let file_name = document
        .file_name
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Check file extension
    let ext = file_name
        .rsplit('.')
        .next()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if !["pdf", "txt", "md", "text"].contains(&ext.as_str()) {
        bot.send_message(
            chat_id,
            "Unsupported file type. Please upload PDF or text files.",
        )
        .await?;
        return Ok(());
    }

    // Send processing message
    let processing_msg = bot
        .send_message(chat_id, format!("Processing document: {}...", file_name))
        .await?;

    // Download the file
    let file = match bot.get_file(&document.file.id).await {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to get file info: {}", e);
            bot.edit_message_text(
                chat_id,
                processing_msg.id,
                "Failed to download document. Please try again.",
            )
            .await?;
            return Ok(());
        }
    };

    // Download file content
    let mut file_content = Vec::new();
    match bot.download_file(&file.path, &mut file_content).await {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to download file content: {}", e);
            bot.edit_message_text(
                chat_id,
                processing_msg.id,
                "Failed to download document content. Please try again.",
            )
            .await?;
            return Ok(());
        }
    }

    // Process the document
    match app_state
        .upload_document
        .execute(&file_content, &file_name)
        .await
    {
        Ok(doc_id) => {
            bot.edit_message_text(
                chat_id,
                processing_msg.id,
                format!(
                    "Document '{}' uploaded successfully!\n\nYou can now ask questions about it.",
                    file_name
                ),
            )
            .await?;
            info!("Document uploaded via Telegram: {} ({})", file_name, doc_id);
        }
        Err(e) => {
            error!("Failed to process document: {}", e);
            bot.edit_message_text(
                chat_id,
                processing_msg.id,
                format!("Failed to process document: {}", e),
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle regular text messages (RAG queries)
async fn handle_text_message(
    bot: Bot,
    msg: teloxide::types::Message,
    app_state: AppState,
    user_sessions: UserSessions,
    max_message_length: usize,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);

    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    // Skip if text starts with / (unrecognized command)
    if text.starts_with('/') {
        bot.send_message(chat_id, "Unknown command. Use /help to see available commands.")
            .await?;
        return Ok(());
    }

    // Validate input
    let validated = match app_state.security.validate_query(text) {
        Ok(v) => v,
        Err(e) => {
            bot.send_message(chat_id, format!("Invalid input: {}", e.error))
                .await?;
            return Ok(());
        }
    };

    // Log injection warnings
    if let Some(warning) = &validated.injection_warning {
        warn!(
            user_id = user_id,
            warning = %warning,
            "Telegram: Potential prompt injection detected"
        );
    }

    // Get or create conversation for this user
    let conv_id = {
        let sessions = user_sessions.read().await;
        sessions.get(&user_id).copied()
    };

    let conv_id = match conv_id {
        Some(id) => id,
        None => {
            // Create new conversation
            match app_state
                .conversation_store
                .create_conversation(Some(&user_id.to_string()), Some("Telegram Chat"))
                .await
            {
                Ok(conv) => {
                    user_sessions.write().await.insert(user_id, conv.id);
                    conv.id
                }
                Err(e) => {
                    error!("Failed to create conversation: {}", e);
                    bot.send_message(chat_id, "An error occurred. Please try again.")
                        .await?;
                    return Ok(());
                }
            }
        }
    };

    // Send typing indicator
    let _ = bot.send_chat_action(chat_id, teloxide::types::ChatAction::Typing).await;

    // Load recent conversation history
    let history = app_state
        .conversation_store
        .get_recent_messages(conv_id, 10)
        .await
        .unwrap_or_default();

    // Convert to ChatMessage format
    let chat_history: Vec<rag_inference::ChatMessage> = history
        .into_iter()
        .map(|m| match m.role {
            MessageRole::User => rag_inference::ChatMessage::user(m.content),
            MessageRole::Assistant => rag_inference::ChatMessage::assistant(m.content),
            _ => rag_inference::ChatMessage::user(m.content),
        })
        .collect();

    // Execute RAG query
    let result = if chat_history.is_empty() {
        app_state.query_documents.execute(&validated.content).await
    } else {
        app_state
            .query_documents
            .execute_with_history(&validated.content, chat_history)
            .await
    };

    match result {
        Ok(query_result) => {
            // Store user message
            let user_msg = RagMessage::user(conv_id, text.to_string());
            if let Err(e) = app_state
                .conversation_store
                .add_message(conv_id, user_msg)
                .await
            {
                error!("Failed to store user message: {}", e);
            }

            // Truncate response if needed
            let response = if query_result.answer.len() > max_message_length {
                let truncated: String = query_result
                    .answer
                    .chars()
                    .take(max_message_length - 20)
                    .collect();
                format!("{}...\n\n(truncated)", truncated)
            } else {
                query_result.answer.clone()
            };

            // Store assistant message
            let assistant_msg = RagMessage::assistant(conv_id, query_result.answer.clone());
            if let Err(e) = app_state
                .conversation_store
                .add_message(conv_id, assistant_msg)
                .await
            {
                error!("Failed to store assistant message: {}", e);
            }

            // Send response
            bot.send_message(chat_id, response).await?;

            // Optionally show sources if there are relevant ones
            let relevant_sources: Vec<_> = query_result
                .sources
                .iter()
                .filter(|s| s.score > 0.3)
                .take(3)
                .collect();

            if !relevant_sources.is_empty() {
                let mut sources_text = String::from("Sources:\n");
                for source in relevant_sources {
                    sources_text.push_str(&format!("- {} ({:.0}%)\n", source.source_file, source.score * 100.0));
                }
                bot.send_message(chat_id, sources_text).await?;
            }
        }
        Err(e) => {
            error!("RAG query failed: {}", e);
            bot.send_message(chat_id, format!("Sorry, I couldn't process your question: {}", e))
                .await?;
        }
    }

    Ok(())
}
