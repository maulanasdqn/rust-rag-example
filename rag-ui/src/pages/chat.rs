use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::api::{
    start_chat_stream_with_history, list_conversations, create_conversation, get_messages,
    add_message, update_conversation, delete_conversation, query_documents,
    ChatEvent, SourceInfo, ConversationInfo, MessageInfo, ChatStreamMessage,
};
use crate::components::{Button, Card};

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub sources: Option<Vec<SourceInfo>>,
    pub is_streaming: bool,
    /// User feedback: Some(1) = thumbs up, Some(-1) = thumbs down, None = none
    pub feedback: Option<i8>,
}

impl From<MessageInfo> for ChatMessage {
    fn from(msg: MessageInfo) -> Self {
        ChatMessage {
            id: Some(msg.id),
            role: msg.role,
            content: msg.content,
            sources: None,
            is_streaming: false,
            feedback: None,
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
    let (editing_conv_id, set_editing_conv_id) = signal(Option::<Uuid>::None);
    let (editing_title, set_editing_title) = signal(String::new());
    let (search_query, set_search_query) = signal(String::new());
    let (search_results, set_search_results) = signal(Option::<crate::api::QueryResponse>::None);
    let (search_loading, set_search_loading) = signal(false);

    // NodeRef for auto-scroll and chat input (keyboard shortcuts)
    let messages_container: NodeRef<leptos::html::Div> = NodeRef::new();
    let chat_input_ref: NodeRef<leptos::html::Textarea> = NodeRef::new();

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

    // Keyboard shortcuts: "/" focus chat input, Ctrl+Enter send
    let input_ref_for_shortcut = chat_input_ref.clone();
    Effect::new(move |_| {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let document = match window.document() {
            Some(d) => d,
            None => return,
        };
        let closure = Closure::wrap(Box::new(move |ev: web_sys::Event| {
            let ev = match ev.dyn_into::<web_sys::KeyboardEvent>() {
                Ok(k) => k,
                Err(_) => return,
            };
            if ev.key() == "/" {
                if let Some(target) = ev.target() {
                    if let Ok(elt) = target.dyn_into::<web_sys::Element>() {
                        let tag = elt.tag_name().to_uppercase();
                        if tag == "INPUT" || tag == "TEXTAREA" {
                            return; // Don't steal "/" when typing in another field
                        }
                    }
                }
                ev.prevent_default();
                if let Some(ta) = input_ref_for_shortcut.get() {
                    let _ = ta.focus();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        let _ = document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
        closure.forget();
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

    let start_rename = move |conv_id: Uuid| {
        if let Some(conv) = conversations.get().into_iter().find(|c| c.id == conv_id) {
            set_editing_conv_id.set(Some(conv_id));
            set_editing_title.set(conv.title);
        }
    };

    let save_rename = move |id: Uuid| {
        let title = editing_title.get();
        set_editing_conv_id.set(None);
        if title.trim().is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            if update_conversation(id, title.trim()).await.is_ok() {
                set_conversations.update(|convs| {
                    if let Some(c) = convs.iter_mut().find(|c| c.id == id) {
                        c.title = title.trim().to_string();
                    }
                });
            }
        });
    };

    let cancel_rename = move |_| {
        set_editing_conv_id.set(None);
    };

    let run_search = move |_| {
        let q = search_query.get();
        if q.trim().is_empty() || search_loading.get() {
            return;
        }
        set_search_loading.set(true);
        set_search_results.set(None);
        let query = q.trim().to_string();
        leptos::task::spawn_local(async move {
            match query_documents(&query).await {
                Ok(res) => set_search_results.set(Some(res)),
                Err(_) => set_search_results.set(None),
            }
            set_search_loading.set(false);
        });
    };

    let ask_in_chat = move |query: String| {
        set_search_results.set(None);
        set_search_query.set(String::new());
        set_input.set(query);
    };

    let export_chat = move |_| {
        let conv_id = current_conversation.get();
        let convs = conversations.get();
        let msgs = messages.get();
        let title = conv_id
            .and_then(|id| convs.iter().find(|c| c.id == id).map(|c| c.title.clone()))
            .unwrap_or_else(|| "export".to_string());
        let mut md = format!("# {}\n\n", title);
        for m in msgs.iter() {
            let role = if m.role == "user" { "**You**" } else { "**Assistant**" };
            md.push_str(&format!("{}\n\n{}\n\n", role, m.content));
        }
        let filename = format!("{}.md", title.replace(|c: char| !c.is_alphanumeric(), "_"));
        download_text(&filename, &md);
    };

    let on_feedback = Callback::new(move |(id, rating): (Option<Uuid>, i8)| {
        set_messages.update(|msgs| {
            if let Some(id) = id {
                if let Some(m) = msgs.iter_mut().find(|m| m.id == Some(id)) {
                    m.feedback = Some(rating);
                }
            }
        });
    });

    let on_suggested_click = Callback::new(move |question: String| {
        set_input.set(question);
        if let Some(ta) = chat_input_ref.get() {
            let _ = ta.focus();
        }
    });

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
                                    let conv_title = conv.title.clone();
                                    let is_selected = move || current_conversation.get() == Some(conv_id);
                                    let is_editing = move || editing_conv_id.get() == Some(conv_id);
                                    view! {
                                        <div class=move || format!(
                                            "group flex items-center gap-2 px-3 py-2 rounded-lg transition-colors {}",
                                            if is_selected() { "bg-primary/10 text-foreground" } else { "hover:bg-muted text-muted-foreground" }
                                        )>
                                            {move || if is_editing() {
                                                view! {
                                                    <input
                                                        type="text"
                                                        class="flex-1 min-w-0 rounded px-2 py-1 text-sm bg-background border border-input text-foreground"
                                                        prop:value=editing_title
                                                        on:input=move |ev| set_editing_title.set(event_target_value(&ev))
                                                        on:keydown=move |ev| {
                                                            if ev.key() == "Enter" { save_rename(conv_id); }
                                                            if ev.key() == "Escape" { cancel_rename(()); }
                                                        }
                                                        on:blur=move |_| save_rename(conv_id)
                                                    />
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        class="flex-1 text-left truncate text-sm cursor-pointer"
                                                        on:click=move |_| select_conversation(conv_id)
                                                    >
                                                        {conv_title.clone()}
                                                    </button>
                                                    <button
                                                        class="opacity-0 group-hover:opacity-100 p-1 hover:bg-muted rounded transition-all"
                                                        on:click=move |_| start_rename(conv_id)
                                                        title="Rename"
                                                    >
                                                        <svg class="w-3 h-3 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path>
                                                        </svg>
                                                    </button>
                                                    <button
                                                        class="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/20 rounded transition-all"
                                                        on:click=move |_| delete_conv(conv_id)
                                                    >
                                                        <svg class="w-3 h-3 text-destructive" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                                        </svg>
                                                    </button>
                                                }.into_any()
                                            }}
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
                    <div class="flex-1 min-w-0">
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
                    <button
                        class="p-2 rounded-lg hover:bg-muted transition-colors text-muted-foreground hover:text-foreground disabled:opacity-50"
                        title="Export as Markdown"
                        disabled=move || current_conversation.get().is_none() || messages.get().is_empty()
                        on:click=export_chat
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                        </svg>
                    </button>
                </div>

                // Search documents (before chat)
                <div class="px-4 pb-2 border-b border-border">
                    <div class="flex gap-2">
                        <input
                            type="text"
                            class="flex-1 rounded-lg border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                            placeholder="Search your documents..."
                            prop:value=search_query
                            on:input=move |ev| set_search_query.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" { run_search(()); }
                            }
                        />
                        <Button on_click=move |_| run_search(()) disabled=search_loading>
                            {move || if search_loading.get() {
                                view! {
                                    <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                }.into_any()
                            } else {
                                view! { <span>"Search"</span> }.into_any()
                            }}
                        </Button>
                    </div>
                    {move || {
                        if let Some(res) = search_results.get() {
                            let query_used = search_query.get();
                            view! {
                                <div class="mt-2 p-3 rounded-lg border border-border bg-muted/30 space-y-2">
                                    <p class="text-sm text-foreground line-clamp-2">{res.answer.clone()}</p>
                                    <p class="text-xs text-muted-foreground">"From " {res.sources.len()} " source(s)"</p>
                                    <div class="flex gap-2">
                                        <Button on_click=move |_| ask_in_chat(query_used.clone())>
                                            "Ask in chat"
                                        </Button>
                                        <button
                                            class="text-xs text-muted-foreground hover:text-foreground"
                                            on:click=move |_| set_search_results.set(None)
                                        >
                                            "Dismiss"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
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
                                        view! {
                                            <MessageBubble
                                                message=msg_clone
                                                on_feedback=Some(on_feedback)
                                                on_suggested_click=Some(on_suggested_click)
                                            />
                                        }
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
                                            if ev.key() == "Enter" && ev.ctrl_key() {
                                                ev.prevent_default();
                                                do_send();
                                            } else if ev.key() == "Enter" && !ev.shift_key() {
                                                ev.prevent_default();
                                                do_send();
                                            }
                                        }
                                        node_ref=chat_input_ref
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
                            <p class="text-xs text-muted-foreground mt-2">"Press Enter or Ctrl+Enter to send, Shift+Enter for new line. Press / to focus chat."</p>
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
            feedback: None,
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
            feedback: None,
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
fn SourceCitations(sources: Vec<SourceInfo>) -> impl IntoView {
    let (expanded, set_expanded) = signal(false);
    view! {
        <div class="pl-11 mt-2">
            <button
                type="button"
                class="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5 transition-colors"
                on:click=move |_| set_expanded.update(|v| *v = !*v)
            >
                <svg class=move || format!("w-3.5 h-3.5 transition-transform {}", if expanded.get() { "rotate-90" } else { "" }) fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                </svg>
                <span>"Cited sources (" {sources.len()} ")"</span>
            </button>
            {move || if expanded.get() {
                view! {
                    <div class="mt-2 space-y-2">
                        {sources.iter().map(|source| {
                            let relevance = (source.relevance_score * 100.0) as i32;
                            view! {
                                <div class="rounded-lg border border-border bg-muted/50 p-3 text-xs">
                                    <div class="flex items-center gap-2 mb-1">
                                        <svg class="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                                        </svg>
                                        <span class="font-medium text-foreground truncate">{source.source_file.clone()}</span>
                                        <span class="text-muted-foreground flex-shrink-0">{format!("{}% match", relevance)}</span>
                                    </div>
                                    <p class="text-muted-foreground line-clamp-3 pl-5">{source.excerpt.clone()}</p>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}
        </div>
    }
}

const SUGGESTED_QUESTIONS: &[&str] = &[
    "Tell me more about this",
    "Summarize briefly",
    "What are the key points?",
];

#[component]
fn MessageBubble(
    message: ChatMessage,
    on_feedback: Option<Callback<(Option<Uuid>, i8)>>,
    on_suggested_click: Option<Callback<String>>,
) -> impl IntoView {
    let is_user = message.role == "user";
    let show_feedback = !is_user && !message.is_streaming && on_feedback.is_some();
    let show_suggestions = !is_user && !message.is_streaming && on_suggested_click.is_some();
    let msg_id = message.id;
    let feedback = message.feedback;

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

                        // Cited sources (expandable with excerpts)
                        {message.sources.clone().map(|sources| {
                            if sources.is_empty() {
                                None
                            } else {
                                Some(view! {
                                    <SourceCitations sources=sources />
                                })
                            }
                        })}

                        // Feedback (thumbs up/down) for assistant messages
                        {move || if show_feedback {
                            let up = move |_| {
                                if let Some(cb) = on_feedback.clone() {
                                    cb.run((msg_id, 1));
                                }
                            };
                            let down = move |_| {
                                if let Some(cb) = on_feedback.clone() {
                                    cb.run((msg_id, -1));
                                }
                            };
                            view! {
                                <div class="flex items-center gap-1 mt-2 pt-2 border-t border-border/50">
                                    <button
                                        type="button"
                                        class=move || format!(
                                            "p-1.5 rounded transition-colors {}",
                                            if feedback == Some(1) { "text-primary bg-primary/10" } else { "text-muted-foreground hover:text-foreground hover:bg-muted" }
                                        )
                                        title="Helpful"
                                        on:click=up
                                    >
                                        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                                            <path d="M2 10.5a1.5 1.5 0 113 0v6a1.5 1.5 0 01-3 0v-6zM6 10.333v5.43a2 2 0 001.106 1.79l.05.025A4 4 0 008.943 18h5.416a2 2 0 001.962-1.608l1.2-6A2 2 0 0015.56 8H12V4a2 2 0 00-2-2 1 1 0 00-1 1v.667a4 4 0 01-.8 2.4L6.8 7.933a4 4 0 00-.8 2.4z"></path>
                                        </svg>
                                    </button>
                                    <button
                                        type="button"
                                        class=move || format!(
                                            "p-1.5 rounded transition-colors {}",
                                            if feedback == Some(-1) { "text-destructive bg-destructive/10" } else { "text-muted-foreground hover:text-foreground hover:bg-muted" }
                                        )
                                        title="Not helpful"
                                        on:click=down
                                    >
                                        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                                            <path d="M18 9.5a1.5 1.5 0 11-3 0 1.5 1.5 0 013 0zM12.5 2c-.828 0-1.5.672-1.5 1.5v7.5c0 .828.672 1.5 1.5 1.5H17c.828 0 1.5-.672 1.5-1.5v-7.5c0-.828-.672-1.5-1.5-1.5h-4.5zM3 13.5A1.5 1.5 0 014.5 12H9v6H4.5A1.5 1.5 0 013 16.5v-3z"></path>
                                        </svg>
                                    </button>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}

                        // Suggested follow-up questions
                        {move || if show_suggestions {
                            let cb = on_suggested_click.clone();
                            view! {
                                <div class="flex flex-wrap gap-1.5 mt-2 pt-2 border-t border-border/50">
                                    {SUGGESTED_QUESTIONS.iter().map(|q| {
                                        let question = (*q).to_string();
                                        let question_for_click = question.clone();
                                        let click = move |_| {
                                            if let Some(ref c) = cb {
                                                c.run(question_for_click.clone());
                                            }
                                        };
                                        view! {
                                            <button
                                                type="button"
                                                class="text-xs px-2.5 py-1 rounded-md border border-border bg-muted/50 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                                                on:click=click
                                            >
                                                {question}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    }.into_any()
                }}
            </div>
        </div>
    }
}

/// Trigger browser download of text content as a file.
fn download_text(filename: &str, content: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let arr = js_sys::Array::new();
    arr.push(&JsValue::from_str(content));
    let opts = {
        let o = web_sys::BlobPropertyBag::new();
        o.set_type("text/markdown");
        o
    };
    let blob = match web_sys::Blob::new_with_str_sequence_and_options(&arr, &opts) {
        Ok(b) => b,
        Err(_) => return,
    };
    let url = match web_sys::Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(_) => return,
    };
    let a: web_sys::HtmlAnchorElement = match document
        .create_element("a")
        .ok()
        .and_then(|e| e.dyn_into().ok())
    {
        Some(anchor) => anchor,
        None => {
            let _ = web_sys::Url::revoke_object_url(&url);
            return;
        }
    };
    let _ = a.set_attribute("href", &url);
    let _ = a.set_attribute("download", filename);
    let _ = a.set_attribute("style", "display: none");
    if let Some(body) = document.body() {
        let _ = body.append_child(&a);
    }
    let _ = a.click();
    let _ = a.remove();
    let _ = web_sys::Url::revoke_object_url(&url);
}
