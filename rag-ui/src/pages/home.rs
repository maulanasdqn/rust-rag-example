use leptos::prelude::*;

use crate::api::{list_documents, query_documents, QueryResponse, SourceInfo};
use crate::components::{Button, Card, StatsCard, Textarea};

#[component]
pub fn HomePage() -> impl IntoView {
    let (question, set_question) = signal(String::new());
    let (result, set_result) = signal(None::<QueryResponse>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);
    let (doc_count, set_doc_count) = signal(0usize);

    // Load document count on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(docs) = list_documents().await {
                set_doc_count.set(docs.documents.len());
            }
        });
    });

    let on_submit = move |_| {
        let q = question.get();
        if q.trim().is_empty() {
            return;
        }

        set_loading.set(true);
        set_error.set(None);
        set_result.set(None);

        leptos::task::spawn_local(async move {
            match query_documents(&q).await {
                Ok(response) => {
                    set_result.set(Some(response));
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            set_loading.set(false);
        });
    };

    view! {
        <div class="space-y-6">
            // Page header
            <div>
                <h1 class="text-2xl font-bold text-foreground">"Dashboard"</h1>
                <p class="text-muted-foreground">"Ask questions and get answers from your documents."</p>
            </div>

            // Stats grid
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                <StatsCard
                    title="Documents"
                    value=move || doc_count.get().to_string()
                    description="Uploaded files"
                    icon="documents"
                />
                <StatsCard
                    title="Chunks"
                    value=|| "785".to_string()
                    description="Indexed segments"
                    icon="chunks"
                />
                <StatsCard
                    title="Search Speed"
                    value=|| "~75ms".to_string()
                    description="In-memory cache"
                    icon="speed"
                />
                <StatsCard
                    title="Model"
                    value=|| "GPT-4o".to_string()
                    description="OpenAI"
                    icon="queries"
                />
            </div>

            // Query section
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                // Query input
                <div class="lg:col-span-2">
                    <Card>
                        <div class="p-6">
                            <h2 class="text-lg font-semibold text-foreground mb-4">"Ask a Question"</h2>
                            <div class="space-y-4">
                                <Textarea
                                    value=question
                                    on_input=move |v| set_question.set(v)
                                    placeholder="What would you like to know about your documents?"
                                    rows=3
                                />
                                <div class="flex items-center justify-between">
                                    <p class="text-xs text-muted-foreground">
                                        "Press Enter or click Ask to search"
                                    </p>
                                    <Button on_click=on_submit disabled=loading>
                                        {move || if loading.get() {
                                            view! {
                                                <span class="flex items-center gap-2">
                                                    <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                    </svg>
                                                    "Searching..."
                                                </span>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <span class="flex items-center gap-2">
                                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                                                    </svg>
                                                    "Ask"
                                                </span>
                                            }.into_any()
                                        }}
                                    </Button>
                                </div>
                            </div>
                        </div>
                    </Card>
                </div>

                // Quick tips
                <div>
                    <Card>
                        <div class="p-6">
                            <h2 class="text-lg font-semibold text-foreground mb-4">"Quick Tips"</h2>
                            <ul class="space-y-3 text-sm text-muted-foreground">
                                <li class="flex items-start gap-2">
                                    <svg class="w-4 h-4 mt-0.5 text-primary flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                    </svg>
                                    "Be specific with your questions"
                                </li>
                                <li class="flex items-start gap-2">
                                    <svg class="w-4 h-4 mt-0.5 text-primary flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                    </svg>
                                    "Upload relevant documents first"
                                </li>
                                <li class="flex items-start gap-2">
                                    <svg class="w-4 h-4 mt-0.5 text-primary flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                    </svg>
                                    "Check sources for context"
                                </li>
                            </ul>
                        </div>
                    </Card>
                </div>
            </div>

            // Error display
            {move || error.get().map(|e| view! {
                <div class="p-4 bg-destructive/10 border border-destructive/20 rounded-xl">
                    <div class="flex items-start gap-3">
                        <svg class="w-5 h-5 text-destructive flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                        </svg>
                        <div>
                            <p class="font-medium text-destructive">"Error"</p>
                            <p class="text-sm text-destructive/80">{e}</p>
                        </div>
                    </div>
                </div>
            })}

            // Results
            {move || result.get().map(|r| view! {
                <div class="space-y-6">
                    // Answer
                    <Card>
                        <div class="p-6">
                            <div class="flex items-center gap-2 mb-4">
                                <div class="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center">
                                    <svg class="w-4 h-4 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
                                    </svg>
                                </div>
                                <h2 class="text-lg font-semibold text-foreground">"Answer"</h2>
                            </div>
                            <p class="text-foreground leading-relaxed">{r.answer.clone()}</p>
                        </div>
                    </Card>

                    // Sources
                    {if !r.sources.is_empty() {
                        Some(view! {
                            <div>
                                <h2 class="text-lg font-semibold text-foreground mb-4">"Sources"</h2>
                                <div class="grid gap-4 md:grid-cols-2">
                                    {r.sources.iter().map(|source| {
                                        view! { <SourceCard source=source.clone() /> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }}
                </div>
            })}
        </div>
    }
}

#[component]
fn SourceCard(source: SourceInfo) -> impl IntoView {
    let relevance_percent = (source.relevance_score * 100.0) as i32;

    view! {
        <Card>
            <div class="p-4">
                <div class="flex items-center justify-between mb-3">
                    <div class="flex items-center gap-2">
                        <svg class="w-4 h-4 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                        </svg>
                        <span class="font-medium text-sm text-foreground truncate max-w-[200px]">{source.source_file}</span>
                    </div>
                    <span class="text-xs px-2 py-1 bg-primary/10 text-primary rounded-full font-medium">
                        {format!("{}%", relevance_percent)}
                    </span>
                </div>
                <p class="text-sm text-muted-foreground line-clamp-3">
                    {source.excerpt}
                </p>
            </div>
        </Card>
    }
}
