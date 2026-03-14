use leptos::prelude::*;

use crate::api::list_documents;
use crate::components::{Card, StatsCard};

#[component]
pub fn HomePage() -> impl IntoView {
    let (doc_count, set_doc_count) = signal(0usize);

    // Load document count on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(docs) = list_documents().await {
                set_doc_count.set(docs.documents.len());
            }
        });
    });

    view! {
        <div class="space-y-6">
            // Page header
            <div>
                <h1 class="text-2xl font-bold text-foreground">"Dashboard"</h1>
                <p class="text-muted-foreground">"Overview of your RAG system."</p>
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

            // Quick actions
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                <a href="/chat" class="block">
                    <Card>
                        <div class="p-6 flex items-center gap-4 hover:bg-muted/50 transition-colors rounded-xl">
                            <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                                <svg class="w-6 h-6 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
                                </svg>
                            </div>
                            <div>
                                <h3 class="font-semibold text-foreground">"Chat"</h3>
                                <p class="text-sm text-muted-foreground">"Ask questions about your documents"</p>
                            </div>
                        </div>
                    </Card>
                </a>

                <a href="/documents" class="block">
                    <Card>
                        <div class="p-6 flex items-center gap-4 hover:bg-muted/50 transition-colors rounded-xl">
                            <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                                <svg class="w-6 h-6 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                                </svg>
                            </div>
                            <div>
                                <h3 class="font-semibold text-foreground">"Documents"</h3>
                                <p class="text-sm text-muted-foreground">"Manage your knowledge base"</p>
                            </div>
                        </div>
                    </Card>
                </a>
            </div>

            // Quick tips
            <Card>
                <div class="p-6">
                    <h2 class="text-lg font-semibold text-foreground mb-4">"Getting Started"</h2>
                    <ul class="space-y-3 text-sm text-muted-foreground">
                        <li class="flex items-start gap-2">
                            <svg class="w-4 h-4 mt-0.5 text-primary flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                            </svg>
                            "Upload documents to build your knowledge base"
                        </li>
                        <li class="flex items-start gap-2">
                            <svg class="w-4 h-4 mt-0.5 text-primary flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                            </svg>
                            "Use Chat for simple Q&A with your documents"
                        </li>
                    </ul>
                </div>
            </Card>
        </div>
    }
}
