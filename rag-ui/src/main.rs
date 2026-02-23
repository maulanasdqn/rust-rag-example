mod api;
mod components;
mod pages;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use components::{MobileNav, Sidebar};
use pages::{ChatPage, DocumentsPage, HomePage};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="min-h-screen bg-background">
                // Sidebar (desktop)
                <Sidebar />

                // Main content area
                <div class="md:pl-64 flex flex-col min-h-screen">
                    // Header
                    <Header />

                    // Page content
                    <main class="flex-1 p-6 pb-20 md:pb-6">
                        <Routes fallback=|| "Page not found">
                            <Route path=path!("/") view=HomePage />
                            <Route path=path!("/chat") view=ChatPage />
                            <Route path=path!("/documents") view=DocumentsPage />
                        </Routes>
                    </main>
                </div>

                // Mobile navigation
                <MobileNav />
            </div>
        </Router>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <header class="sticky top-0 z-40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b border-border">
            <div class="flex h-16 items-center justify-between px-6">
                <div class="flex items-center gap-4">
                    <h1 class="text-lg font-semibold text-foreground md:hidden">"RAG Assistant"</h1>
                </div>
                <div class="flex items-center gap-4">
                    // Search (optional)
                    <div class="hidden md:flex items-center gap-2 px-3 py-1.5 rounded-lg bg-muted text-muted-foreground text-sm">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                        </svg>
                        <span>"Search..."</span>
                        <kbd class="ml-2 px-1.5 py-0.5 text-xs bg-background rounded border border-border">"/"</kbd>
                    </div>

                    // Theme toggle placeholder
                    <button class="p-2 rounded-lg hover:bg-muted transition-colors">
                        <svg class="w-5 h-5 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"></path>
                        </svg>
                    </button>
                </div>
            </div>
        </header>
    }
}
