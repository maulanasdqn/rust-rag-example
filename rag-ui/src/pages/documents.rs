use leptos::prelude::*;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::{FormData, HtmlInputElement};

use crate::api::{delete_document, list_documents, upload_document, DocumentInfo};
use crate::components::{Button, Card};

#[component]
pub fn DocumentsPage() -> impl IntoView {
    let (documents, set_documents) = signal(Vec::<DocumentInfo>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (uploading, set_uploading) = signal(false);
    let (upload_message, set_upload_message) = signal(None::<String>);
    let (drag_over, set_drag_over) = signal(false);

    // Load documents on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match list_documents().await {
                Ok(response) => {
                    set_documents.set(response.documents);
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            set_loading.set(false);
        });
    });

    let refresh_documents = move || {
        set_loading.set(true);
        leptos::task::spawn_local(async move {
            match list_documents().await {
                Ok(response) => {
                    set_documents.set(response.documents);
                    set_error.set(None);
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            set_loading.set(false);
        });
    };

    let on_file_upload = move |ev: leptos::ev::Event| {
        let target = ev.target().unwrap();
        let input: HtmlInputElement = target.dyn_into().unwrap();

        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                set_uploading.set(true);
                set_upload_message.set(None);
                set_error.set(None);

                leptos::task::spawn_local(async move {
                    let form_data = FormData::new().unwrap();
                    form_data.append_with_blob("file", &file).unwrap();

                    match upload_document(form_data).await {
                        Ok(response) => {
                            set_upload_message.set(Some(response.message));
                            if let Ok(docs) = list_documents().await {
                                set_documents.set(docs.documents);
                            }
                        }
                        Err(e) => {
                            set_error.set(Some(e));
                        }
                    }
                    set_uploading.set(false);
                });
            }
        }
    };

    let on_delete = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            match delete_document(id).await {
                Ok(_) => {
                    if let Ok(docs) = list_documents().await {
                        set_documents.set(docs.documents);
                    }
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
        });
    };

    view! {
        <div class="space-y-6">
            // Page header
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-2xl font-bold text-foreground">"Documents"</h1>
                    <p class="text-muted-foreground">"Upload and manage your knowledge base."</p>
                </div>
                <Button on_click=move |_| refresh_documents() disabled=loading>
                    <span class="flex items-center gap-2">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                        </svg>
                        "Refresh"
                    </span>
                </Button>
            </div>

            // Upload area
            <Card>
                <div class="p-6">
                    <div
                        class=move || format!(
                            "border-2 border-dashed rounded-xl p-8 text-center transition-colors {}",
                            if drag_over.get() { "border-primary bg-primary/5" } else { "border-border hover:border-primary/50" }
                        )
                        on:dragover=move |ev| {
                            ev.prevent_default();
                            set_drag_over.set(true);
                        }
                        on:dragleave=move |_| set_drag_over.set(false)
                        on:drop=move |ev| {
                            ev.prevent_default();
                            set_drag_over.set(false);
                        }
                    >
                        <div class="flex flex-col items-center gap-4">
                            <div class="w-12 h-12 rounded-full bg-muted flex items-center justify-center">
                                <svg class="w-6 h-6 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
                                </svg>
                            </div>
                            <div>
                                <p class="text-foreground font-medium">"Upload a document"</p>
                                <p class="text-sm text-muted-foreground">"PDF, TXT, or Markdown files"</p>
                            </div>
                            <label class="cursor-pointer">
                                <input
                                    type="file"
                                    accept=".pdf,.txt,.md"
                                    on:change=on_file_upload
                                    disabled=uploading
                                    class="hidden"
                                />
                                <span class="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg font-medium text-sm hover:bg-primary/90 transition-colors">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                                    </svg>
                                    "Choose File"
                                </span>
                            </label>
                        </div>
                    </div>

                    // Upload progress
                    {move || uploading.get().then(|| view! {
                        <div class="mt-4 space-y-2">
                            <div class="flex items-center gap-2">
                                <div class="flex-1 h-2 bg-muted rounded-full overflow-hidden">
                                    <div class="h-full bg-primary rounded-full animate-pulse w-full"></div>
                                </div>
                            </div>
                            <p class="text-sm text-muted-foreground text-center">"Processing document..."</p>
                        </div>
                    })}

                    // Success message
                    {move || upload_message.get().map(|msg| view! {
                        <div class="mt-4 p-3 bg-green-500/10 border border-green-500/20 rounded-lg">
                            <div class="flex items-center gap-2 text-green-600">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                </svg>
                                <span class="text-sm font-medium">{msg}</span>
                            </div>
                        </div>
                    })}
                </div>
            </Card>

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

            // Documents list
            <Card>
                <div class="p-6">
                    <h2 class="text-lg font-semibold text-foreground mb-4">"Uploaded Documents"</h2>

                    {move || {
                        if loading.get() {
                            view! {
                                <div class="flex items-center justify-center py-12">
                                    <svg class="animate-spin h-8 w-8 text-muted-foreground" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                </div>
                            }.into_any()
                        } else if documents.get().is_empty() {
                            view! {
                                <div class="text-center py-12">
                                    <div class="w-16 h-16 mx-auto rounded-full bg-muted flex items-center justify-center mb-4">
                                        <svg class="w-8 h-8 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                                        </svg>
                                    </div>
                                    <p class="text-foreground font-medium">"No documents yet"</p>
                                    <p class="text-sm text-muted-foreground mt-1">"Upload a document to get started."</p>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-2">
                                    {documents.get().iter().map(|doc| {
                                        let id = doc.id;
                                        let name = doc.name.clone();
                                        view! {
                                            <div class="flex items-center justify-between p-4 rounded-lg bg-muted/50 hover:bg-muted transition-colors group">
                                                <div class="flex items-center gap-3 min-w-0">
                                                    <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
                                                        <svg class="w-5 h-5 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                                                        </svg>
                                                    </div>
                                                    <div class="min-w-0">
                                                        <p class="font-medium text-foreground truncate">{name}</p>
                                                        <p class="text-xs text-muted-foreground">{format!("ID: {}...", &id.to_string()[..8])}</p>
                                                    </div>
                                                </div>
                                                <button
                                                    on:click=move |_| on_delete(id)
                                                    class="p-2 rounded-lg text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors opacity-0 group-hover:opacity-100"
                                                >
                                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                                    </svg>
                                                </button>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </Card>
        </div>
    }
}
