mod api;
mod components;
mod pages;

use leptos::prelude::*;
use leptos::prelude::Effect;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use components::{MobileNav, Sidebar};
use pages::{ChatPage, DocumentsPage, HomePage};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

const THEME_STORAGE_KEY: &str = "rag-ui-theme";

#[component]
fn App() -> impl IntoView {
    let (theme, set_theme) = signal::<String>("dark".to_string());

    // Restore theme from localStorage on mount
    let _ = Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(stored)) = storage.get_item(THEME_STORAGE_KEY) {
                    set_theme.set(stored);
                }
            }
        }
    });

    // Persist theme and apply class when it changes
    let _ = Effect::new(move |_| {
        let t = theme.get();
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(THEME_STORAGE_KEY, &t);
            }
            if let Some(doc) = window.document() {
                if let Some(html) = doc.document_element() {
                    if t == "light" {
                        let _ = html.class_list().add_1("light");
                    } else {
                        let _ = html.class_list().remove_1("light");
                    }
                }
            }
        }
    });

    provide_context(theme);
    provide_context(set_theme);

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
    let set_theme = use_context::<WriteSignal<String>>().expect("theme context");
    let theme = use_context::<ReadSignal<String>>().expect("theme context");

    let toggle_theme = move |_| {
        set_theme.set(if theme.get() == "light" {
            "dark".to_string()
        } else {
            "light".to_string()
        });
    };

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

                    // Theme toggle
                    <button
                        class="p-2 rounded-lg hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
                        title=move || if theme.get() == "light" { "Switch to dark mode" } else { "Switch to light mode" }
                        on:click=toggle_theme
                    >
                        {move || if theme.get() == "light" {
                            view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"></path>
                                </svg>
                            }.into_any()
                        } else {
                            view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"></path>
                                </svg>
                            }.into_any()
                        }}
                    </button>
                </div>
            </div>
        </header>
    }
}
