use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

use crate::api::{
    start_chat_stream_with_history, list_conversations, create_conversation, get_messages,
    add_message, delete_conversation, ChatEvent, SourceInfo, ConversationInfo, MessageInfo,
    ChatStreamMessage,
};
use crate::components::{Button, Card};

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub sources: Option<Vec<SourceInfo>>,
    pub is_streaming: bool,
}

impl From<MessageInfo> for ChatMessage {
    fn from(msg: MessageInfo) -> Self {
        ChatMessage {
            id: Some(msg.id),
            role: msg.role,
            content: msg.content,
            sources: None,
            is_streaming: false,
        }
    }
}

#[component]
pub fn ChatPage() -> impl IntoView {
    let (conversations, set_conversations) = signal(Vec::<ConversationInfo>::new());
    let (current_conversation, set_current_conversation) = signal(Option::<Uuid>::None);
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input, set_input) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (sidebar_open, set_sidebar_open) = signal(true);

    // NodeRef for auto-scroll
    let messages_container: NodeRef<leptos::html::Div> = NodeRef::new();

    // Auto-scroll to bottom when messages change
    Effect::new(move |_| {
        // Track messages changes
        let _ = messages.get();

        // Scroll to bottom using requestAnimationFrame to ensure DOM is updated
        if let Some(container) = messages_container.get() {
            let container_el: web_sys::Element = container.clone().into();
            // Use requestAnimationFrame to wait for DOM update
            let closure = Closure::once(Box::new(move || {
                container_el.set_scroll_top(container_el.scroll_height());
            }) as Box<dyn FnOnce()>);

            if let Some(window) = web_sys::window() {
                let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                closure.forget(); // Prevent closure from being dropped
            }
        }
    });

    // Load conversations on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(response) = list_conversations().await {
                set_conversations.set(response.conversations);
            }
        });
    });

    // Function to load messages for a conversation
    let load_messages = move |conv_id: Uuid| {
        leptos::task::spawn_local(async move {
            match get_messages(conv_id).await {
                Ok(response) => {
                    let msgs: Vec<ChatMessage> = response.messages.into_iter().map(ChatMessage::from).collect();
                    web_sys::console::log_1(&format!("Loaded {} messages for conversation {}", msgs.len(), conv_id).into());
                    set_messages.set(msgs);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to load messages: {}", e).into());
                }
            }
        });
    };

    let create_new_conversation = move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(conv) = create_conversation(Some("New Chat".to_string())).await {
                let conv_id = conv.id;
                set_conversations.update(|convs| convs.insert(0, conv));
                set_current_conversation.set(Some(conv_id));
                set_messages.set(Vec::new());
            }
        });
    };

    let select_conversation = move |id: Uuid| {
        web_sys::console::log_1(&format!("Selecting conversation: {}", id).into());
        set_current_conversation.set(Some(id));
        set_messages.set(Vec::new()); // Clear first to show loading state
        load_messages(id); // Load messages for selected conversation
    };

    let delete_conv = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            if delete_conversation(id).await.is_ok() {
                set_conversations.update(|convs| convs.retain(|c| c.id != id));
                if current_conversation.get() == Some(id) {
                    set_current_conversation.set(None);
                    set_messages.set(Vec::new());
                }
            }
        });
    };

    let do_send = move || {
        let question = input.get();
        if question.trim().is_empty() || is_loading.get() {
            return;
        }

        let conv_id = current_conversation.get();

        // If no conversation selected, create one first then send
        if conv_id.is_none() {
            let question_clone = question.clone();
            set_input.set(String::new());

            leptos::task::spawn_local(async move {
                if let Ok(conv) = create_conversation(Some(question_clone.chars().take(50).collect())).await {
                    let conv_id = conv.id;
                    set_conversations.update(|convs| convs.insert(0, conv));
                    set_current_conversation.set(Some(conv_id));
                    set_messages.set(Vec::new());

                    // Now send the message (empty history for new conversation)
                    send_message(conv_id, question_clone, Vec::new(), set_messages, set_is_loading, set_conversations).await;
                }
            });
            return;
        }

        let conv_id = conv_id.unwrap();
        let question_clone = question.clone();
        let current_msgs = messages.get();
        set_input.set(String::new());

        leptos::task::spawn_local(async move {
            send_message(conv_id, question_clone, current_msgs, set_messages, set_is_loading, set_conversations).await;
        });
    };

    view! {
        <div class="flex h-[calc(100vh-8rem)]">
            // Sidebar with conversation history
            <div class=move || format!(
                "flex-shrink-0 border-r border-border bg-card transition-all duration-300 {}",
                if sidebar_open.get() { "w-64" } else { "w-0 overflow-hidden" }
            )>
                <div class="flex flex-col h-full">
                    // Sidebar header
                    <div class="p-3 border-b border-border">
                        <button
                            class="w-full flex items-center justify-center gap-2 px-3 py-2 text-sm font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
                            on:click=create_new_conversation
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                            </svg>
                            "New Chat"
                        </button>
                    </div>

                    // Conversation list
                    <div class="flex-1 overflow-y-auto p-2 space-y-1">
                        {move || {
                            let convs = conversations.get();
                            if convs.is_empty() {
                                view! {
                                    <p class="text-sm text-muted-foreground text-center py-4">"No conversations yet"</p>
                                }.into_any()
                            } else {
                                convs.into_iter().map(|conv| {
                                    let conv_id = conv.id;
                                    let is_selected = move || current_conversation.get() == Some(conv_id);
                                    view! {
                                        <div class=move || format!(
                                            "group flex items-center gap-2 px-3 py-2 rounded-lg cursor-pointer transition-colors {}",
                                            if is_selected() { "bg-primary/10 text-foreground" } else { "hover:bg-muted text-muted-foreground" }
                                        )>
                                            <button
                                                class="flex-1 text-left truncate text-sm"
                                                on:click=move |_| select_conversation(conv_id)
                                            >
                                                {conv.title.clone()}
                                            </button>
                                            <button
                                                class="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/20 rounded transition-all"
                                                on:click=move |_| delete_conv(conv_id)
                                            >
                                                <svg class="w-3 h-3 text-destructive" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                                </svg>
                                            </button>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Main chat area
            <div class="flex-1 flex flex-col min-w-0">
                // Header
                <div class="flex items-center gap-3 p-4 border-b border-border">
                    <button
                        class="p-2 hover:bg-muted rounded-lg transition-colors"
                        on:click=move |_| set_sidebar_open.update(|v| *v = !*v)
                    >
                        <svg class="w-5 h-5 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path>
                        </svg>
                    </button>
                    <div>
                        <h1 class="text-lg font-semibold text-foreground">"Chat"</h1>
                        <p class="text-xs text-muted-foreground">
                            {move || {
                                if let Some(conv_id) = current_conversation.get() {
                                    conversations.get().iter().find(|c| c.id == conv_id)
                                        .map(|c| c.title.clone())
                                        .unwrap_or_else(|| "Select a conversation".to_string())
                                } else {
                                    "Start a new conversation".to_string()
                                }
                            }}
                        </p>
                    </div>
                </div>

                // Messages area
                <div node_ref=messages_container class="flex-1 overflow-y-auto p-4 space-y-4">
                    {move || {
                        let msgs = messages.get();
                        if msgs.is_empty() {
                            view! {
                                <div class="flex items-center justify-center h-full">
                                    <div class="text-center max-w-md">
                                        <div class="w-16 h-16 mx-auto rounded-full bg-primary/10 flex items-center justify-center mb-4">
                                            <svg class="w-8 h-8 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
                                            </svg>
                                        </div>
                                        <h3 class="text-lg font-semibold text-foreground mb-2">"Start a conversation"</h3>
                                        <p class="text-muted-foreground text-sm">"Ask questions about your documents. Your chat history will be saved."</p>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-4">
                                    {msgs.iter().map(|msg| {
                                        let msg_clone = msg.clone();
                                        view! { <MessageBubble message=msg_clone /> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>

                // Input area
                <div class="p-4 border-t border-border">
                    <Card>
                        <div class="p-3">
                            <div class="flex gap-3">
                                <div class="flex-1">
                                    <textarea
                                        prop:value=move || input.get()
                                        on:input=move |ev| {
                                            set_input.set(event_target_value(&ev));
                                        }
                                        on:keydown=move |ev| {
                                            if ev.key() == "Enter" && !ev.shift_key() {
                                                ev.prevent_default();
                                                do_send();
                                            }
                                        }
                                        placeholder="Ask a question about your documents..."
                                        rows=2
                                        class="flex min-h-[60px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm \
                                               ring-offset-background placeholder:text-muted-foreground \
                                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
                                               disabled:cursor-not-allowed disabled:opacity-50 resize-none"
                                    />
                                </div>
                                <div class="flex items-end">
                                    <Button on_click=move |_| do_send() disabled=is_loading>
                                        {move || if is_loading.get() {
                                            view! {
                                                <svg class="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
                                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                </svg>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path>
                                                </svg>
                                            }.into_any()
                                        }}
                                    </Button>
                                </div>
                            </div>
                            <p class="text-xs text-muted-foreground mt-2">"Press Enter to send, Shift+Enter for new line"</p>
                        </div>
                    </Card>
                </div>
            </div>
        </div>
    }
}

async fn send_message(
    conv_id: Uuid,
    question: String,
    current_messages: Vec<ChatMessage>,
    set_messages: WriteSignal<Vec<ChatMessage>>,
    set_is_loading: WriteSignal<bool>,
    set_conversations: WriteSignal<Vec<ConversationInfo>>,
) {
    set_is_loading.set(true);

    // Add user message to UI
    set_messages.update(|msgs| {
        msgs.push(ChatMessage {
            id: None,
            role: "user".to_string(),
            content: question.clone(),
            sources: None,
            is_streaming: false,
        });
    });

    // Save user message to backend
    match add_message(conv_id, "user", &question).await {
        Ok(_) => web_sys::console::log_1(&"User message saved to backend".into()),
        Err(e) => web_sys::console::error_1(&format!("Failed to save user message: {}", e).into()),
    }

    // Collect conversation history for context (from previous messages + new user message)
    let mut history: Vec<ChatStreamMessage> = current_messages
        .iter()
        .filter(|m| !m.is_streaming)
        .map(|m| ChatStreamMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();
    // Add the current user message to history
    history.push(ChatStreamMessage {
        role: "user".to_string(),
        content: question.clone(),
    });

    // Add placeholder for assistant message
    // Use a unique ID to identify this message instead of relying on array index
    let assistant_msg_id = Uuid::new_v4();
    set_messages.update(|msgs| {
        msgs.push(ChatMessage {
            id: Some(assistant_msg_id),
            role: "assistant".to_string(),
            content: String::new(),
            sources: None,
            is_streaming: true,
        });
    });

    // Use Rc<RefCell> to share state in the closure
    let accumulated_content = Rc::new(RefCell::new(String::new()));
    let final_content = Rc::new(RefCell::new(String::new()));

    let content_rc = accumulated_content.clone();
    let final_content_clone = final_content.clone();

    let result = start_chat_stream_with_history(&question, history, move |event| {
        match event {
            ChatEvent::Sources(sources) => {
                set_messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.id == Some(assistant_msg_id)) {
                        msg.sources = Some(sources);
                    }
                });
            }
            ChatEvent::Content(chunk) => {
                content_rc.borrow_mut().push_str(&chunk);
                let new_content = content_rc.borrow().clone();
                *final_content_clone.borrow_mut() = new_content.clone();
                set_messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.id == Some(assistant_msg_id)) {
                        msg.content = new_content;
                    }
                });
            }
            ChatEvent::Done => {
                set_messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.id == Some(assistant_msg_id)) {
                        msg.is_streaming = false;
                    }
                });
                set_is_loading.set(false);
            }
            ChatEvent::Error(error) => {
                set_messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.id == Some(assistant_msg_id)) {
                        msg.content = format!("Error: {}", error);
                        msg.is_streaming = false;
                    }
                });
                set_is_loading.set(false);
            }
        }
    })
    .await;

    // Save assistant message to backend
    let final_response = final_content.borrow().clone();
    if !final_response.is_empty() {
        match add_message(conv_id, "assistant", &final_response).await {
            Ok(_) => web_sys::console::log_1(&"Assistant message saved to backend".into()),
            Err(e) => web_sys::console::error_1(&format!("Failed to save assistant message: {}", e).into()),
        }
        // Update conversation in list
        set_conversations.update(|convs| {
            if let Some(conv) = convs.iter_mut().find(|c| c.id == conv_id) {
                conv.message_count += 2;
            }
        });
    }

    if let Err(e) = result {
        set_messages.update(|msgs| {
            if let Some(msg) = msgs.iter_mut().find(|m| m.id == Some(assistant_msg_id)) {
                msg.content = format!("Error: {}", e);
                msg.is_streaming = false;
            }
        });
        set_is_loading.set(false);
    }
}

#[component]
fn MessageBubble(message: ChatMessage) -> impl IntoView {
    let is_user = message.role == "user";

    view! {
        <div class=move || format!(
            "flex {}",
            if is_user { "justify-end" } else { "justify-start" }
        )>
            <div class=move || format!(
                "max-w-[80%] {}",
                if is_user {
                    "bg-primary text-primary-foreground rounded-2xl rounded-br-md px-4 py-3"
                } else {
                    "space-y-3"
                }
            )>
                {if is_user {
                    view! { <p class="whitespace-pre-wrap">{message.content.clone()}</p> }.into_any()
                } else {
                    view! {
                        <div class="bg-card border border-border rounded-2xl rounded-bl-md px-4 py-3">
                            <div class="flex items-start gap-3">
                                <div class="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
                                    <svg class="w-4 h-4 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
                                    </svg>
                                </div>
                                <div class="flex-1 min-w-0">
                                    <p class="text-foreground whitespace-pre-wrap">
                                        {message.content.clone()}
                                        {if message.is_streaming {
                                            view! { <span class="inline-block w-2 h-4 bg-primary animate-pulse ml-1"></span> }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </p>
                                </div>
                            </div>
                        </div>

                        // Sources
                        {message.sources.clone().map(|sources| {
                            if sources.is_empty() {
                                None
                            } else {
                                Some(view! {
                                    <div class="pl-11">
                                        <p class="text-xs text-muted-foreground mb-2">"Sources:"</p>
                                        <div class="flex flex-wrap gap-2">
                                            {sources.iter().map(|source| {
                                                let relevance = (source.relevance_score * 100.0) as i32;
                                                view! {
                                                    <div class="inline-flex items-center gap-1.5 px-2 py-1 bg-muted rounded-lg text-xs">
                                                        <svg class="w-3 h-3 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                                                        </svg>
                                                        <span class="text-foreground truncate max-w-[150px]">{source.source_file.clone()}</span>
                                                        <span class="text-muted-foreground">{format!("{}%", relevance)}</span>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                })
                            }
                        })}
                    }.into_any()
                }}
            </div>
        </div>
    }
}
