mod api;
mod components;
mod pages;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use pages::{DocumentsPage, HomePage};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="min-h-screen bg-background">
                <Nav />
                <main class="container mx-auto px-4 py-8">
                    <Routes fallback=|| "Page not found">
                        <Route path=path!("/") view=HomePage />
                        <Route path=path!("/documents") view=DocumentsPage />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}

#[component]
fn Nav() -> impl IntoView {
    view! {
        <nav class="border-b border-border bg-card">
            <div class="container mx-auto px-4">
                <div class="flex h-16 items-center justify-between">
                    <div class="flex items-center gap-8">
                        <a href="/" class="text-xl font-bold text-foreground">
                            "RAG UI"
                        </a>
                        <div class="flex gap-4">
                            <a
                                href="/"
                                class="text-muted-foreground hover:text-foreground transition-colors"
                            >
                                "Query"
                            </a>
                            <a
                                href="/documents"
                                class="text-muted-foreground hover:text-foreground transition-colors"
                            >
                                "Documents"
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        </nav>
    }
}
