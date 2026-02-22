use leptos::prelude::*;

use crate::api::{query_documents, QueryResponse, SourceInfo};
use crate::components::{Button, Card, Textarea};

#[component]
pub fn HomePage() -> impl IntoView {
    let (question, set_question) = signal(String::new());
    let (result, set_result) = signal(None::<QueryResponse>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

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
        <div class="max-w-4xl mx-auto space-y-8">
            <div class="text-center space-y-4">
                <h1 class="text-4xl font-bold text-foreground">"Ask Your Documents"</h1>
                <p class="text-muted-foreground text-lg">
                    "Enter a question and get answers from your uploaded documents using RAG."
                </p>
            </div>

            <Card>
                <div class="p-6 space-y-4">
                    <Textarea
                        value=question
                        on_input=move |v| set_question.set(v)
                        placeholder="Enter your question here..."
                        rows=4
                    />
                    <div class="flex justify-end">
                        <Button on_click=on_submit disabled=loading>
                            {move || if loading.get() { "Searching..." } else { "Ask Question" }}
                        </Button>
                    </div>
                </div>
            </Card>

            {move || error.get().map(|e| view! {
                <div class="p-4 bg-destructive/10 border border-destructive rounded-lg text-destructive">
                    <p class="font-medium">"Error"</p>
                    <p>{e}</p>
                </div>
            })}

            {move || result.get().map(|r| view! {
                <div class="space-y-6">
                    <Card>
                        <div class="p-6">
                            <h2 class="text-xl font-semibold mb-4 text-foreground">"Answer"</h2>
                            <p class="text-foreground whitespace-pre-wrap">{r.answer.clone()}</p>
                        </div>
                    </Card>

                    {if !r.sources.is_empty() {
                        Some(view! {
                            <div class="space-y-4">
                                <h2 class="text-xl font-semibold text-foreground">"Sources"</h2>
                                <div class="grid gap-4">
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
            <div class="p-4 space-y-3">
                <div class="flex items-center justify-between">
                    <span class="font-medium text-foreground">{source.source_file}</span>
                    <span class="text-sm px-2 py-1 bg-primary/10 text-primary rounded">
                        {format!("{}% relevant", relevance_percent)}
                    </span>
                </div>
                <p class="text-muted-foreground text-sm italic">
                    "\""
                    {source.excerpt}
                    "\""
                </p>
            </div>
        </Card>
    }
}
