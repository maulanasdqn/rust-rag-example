use leptos::prelude::*;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::{FormData, HtmlInputElement};

use crate::api::{delete_document, list_documents, upload_document};
use crate::components::{Button, Card};

#[component]
pub fn DocumentsPage() -> impl IntoView {
    let (documents, set_documents) = signal(Vec::<Uuid>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (uploading, set_uploading) = signal(false);
    let (upload_message, set_upload_message) = signal(None::<String>);

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
                            // Refresh document list
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
                    // Refresh document list
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
        <div class="max-w-4xl mx-auto space-y-8">
            <div class="text-center space-y-4">
                <h1 class="text-4xl font-bold text-foreground">"Documents"</h1>
                <p class="text-muted-foreground text-lg">
                    "Upload and manage your documents for RAG queries."
                </p>
            </div>

            <Card>
                <div class="p-6 space-y-4">
                    <h2 class="text-xl font-semibold text-foreground">"Upload Document"</h2>
                    <div class="flex items-center gap-4">
                        <input
                            type="file"
                            accept=".pdf,.txt,.md"
                            on:change=on_file_upload
                            disabled=uploading
                            class="flex-1 file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:font-semibold file:bg-primary file:text-primary-foreground hover:file:bg-primary/90 cursor-pointer"
                        />
                    </div>
                    <p class="text-sm text-muted-foreground">
                        "Supported formats: PDF, TXT, Markdown"
                    </p>

                    {move || uploading.get().then(|| view! {
                        <p class="text-primary">"Uploading..."</p>
                    })}

                    {move || upload_message.get().map(|msg| view! {
                        <p class="text-green-600">{msg}</p>
                    })}
                </div>
            </Card>

            {move || error.get().map(|e| view! {
                <div class="p-4 bg-destructive/10 border border-destructive rounded-lg text-destructive">
                    <p class="font-medium">"Error"</p>
                    <p>{e}</p>
                </div>
            })}

            <Card>
                <div class="p-6 space-y-4">
                    <div class="flex items-center justify-between">
                        <h2 class="text-xl font-semibold text-foreground">"Uploaded Documents"</h2>
                        <Button on_click=move |_| refresh_documents() disabled=loading>
                            "Refresh"
                        </Button>
                    </div>

                    {move || {
                        if loading.get() {
                            view! { <p class="text-muted-foreground">"Loading..."</p> }.into_any()
                        } else if documents.get().is_empty() {
                            view! {
                                <p class="text-muted-foreground py-8 text-center">
                                    "No documents uploaded yet. Upload a document to get started."
                                </p>
                            }.into_any()
                        } else {
                            view! {
                                <div class="divide-y divide-border">
                                    {documents.get().iter().map(|id| {
                                        let id = *id;
                                        view! {
                                            <div class="py-3 flex items-center justify-between">
                                                <code class="text-sm font-mono text-foreground">
                                                    {id.to_string()}
                                                </code>
                                                <button
                                                    on:click=move |_| on_delete(id)
                                                    class="text-destructive hover:text-destructive/80 text-sm font-medium"
                                                >
                                                    "Delete"
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
