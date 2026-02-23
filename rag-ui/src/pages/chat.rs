use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::{start_chat_stream, ChatEvent, SourceInfo};
use crate::components::{Button, Card};

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub sources: Option<Vec<SourceInfo>>,
    pub is_streaming: bool,
}

#[component]
pub fn ChatPage() -> impl IntoView {
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input, set_input) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);

    let do_send = move || {
        let question = input.get();
        if question.trim().is_empty() || is_loading.get() {
            return;
        }

        set_input.set(String::new());
        set_is_loading.set(true);

        // Add user message
        set_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: "user".to_string(),
                content: question.clone(),
                sources: None,
                is_streaming: false,
            });
        });

        // Add placeholder for assistant message
        let assistant_idx = messages.get().len();
        set_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: "assistant".to_string(),
                content: String::new(),
                sources: None,
                is_streaming: true,
            });
        });

        let question_clone = question.clone();

        // Use Rc<RefCell> to share state in the closure
        let accumulated_content = Rc::new(RefCell::new(String::new()));
        let accumulated_sources: Rc<RefCell<Option<Vec<SourceInfo>>>> = Rc::new(RefCell::new(None));

        leptos::task::spawn_local(async move {
            let content_rc = accumulated_content.clone();
            let sources_rc = accumulated_sources.clone();

            let result = start_chat_stream(&question_clone, move |event| {
                match event {
                    ChatEvent::Sources(sources) => {
                        *sources_rc.borrow_mut() = Some(sources.clone());
                        set_messages.update(|msgs| {
                            if let Some(msg) = msgs.get_mut(assistant_idx) {
                                msg.sources = Some(sources);
                            }
                        });
                    }
                    ChatEvent::Content(chunk) => {
                        content_rc.borrow_mut().push_str(&chunk);
                        let new_content = content_rc.borrow().clone();
                        set_messages.update(|msgs| {
                            if let Some(msg) = msgs.get_mut(assistant_idx) {
                                msg.content = new_content;
                            }
                        });
                    }
                    ChatEvent::Done => {
                        set_messages.update(|msgs| {
                            if let Some(msg) = msgs.get_mut(assistant_idx) {
                                msg.is_streaming = false;
                            }
                        });
                        set_is_loading.set(false);
                    }
                    ChatEvent::Error(error) => {
                        set_messages.update(|msgs| {
                            if let Some(msg) = msgs.get_mut(assistant_idx) {
                                msg.content = format!("Error: {}", error);
                                msg.is_streaming = false;
                            }
                        });
                        set_is_loading.set(false);
                    }
                }
            })
            .await;

            if let Err(e) = result {
                set_messages.update(|msgs| {
                    if let Some(msg) = msgs.get_mut(assistant_idx) {
                        msg.content = format!("Error: {}", e);
                        msg.is_streaming = false;
                    }
                });
                set_is_loading.set(false);
            }
        });
    };

    view! {
        <div class="flex flex-col h-[calc(100vh-8rem)]">
            // Page header
            <div class="mb-4">
                <h1 class="text-2xl font-bold text-foreground">"Chat"</h1>
                <p class="text-muted-foreground">"Have a conversation with your documents using AI."</p>
            </div>

            // Chat messages area
            <div class="flex-1 overflow-y-auto space-y-4 mb-4 pr-2">
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
                                    <p class="text-muted-foreground text-sm">"Ask questions about your uploaded documents. The AI will search through your knowledge base and provide answers with sources."</p>
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
            <Card>
                <div class="p-4">
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
