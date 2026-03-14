use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <aside class="hidden md:flex md:w-64 md:flex-col md:fixed md:inset-y-0 z-50">
            <div class="flex flex-col flex-grow bg-card border-r border-border overflow-y-auto">
                // Logo
                <div class="flex items-center h-16 flex-shrink-0 px-4 border-b border-border">
                    <div class="flex items-center gap-2">
                        <div class="w-8 h-8 rounded-lg bg-primary flex items-center justify-center">
                            <svg class="w-5 h-5 text-primary-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                            </svg>
                        </div>
                        <span class="text-xl font-bold text-foreground">"RAG Assistant"</span>
                    </div>
                </div>

                // Navigation
                <nav class="flex-1 px-4 py-6 space-y-2">
                    <SidebarLink href="/" icon="dashboard" label="Dashboard" />
                    <SidebarLink href="/chat" icon="chat" label="Chat" />
                    <SidebarLink href="/documents" icon="documents" label="Documents" />
                </nav>

                // Footer
                <div class="flex-shrink-0 p-4 border-t border-border">
                    <div class="flex items-center gap-3">
                        <div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
                            <svg class="w-4 h-4 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path>
                            </svg>
                        </div>
                        <div class="flex-1 min-w-0">
                            <p class="text-sm font-medium text-foreground truncate">"User"</p>
                            <p class="text-xs text-muted-foreground truncate">"RAG System"</p>
                        </div>
                    </div>
                </div>
            </div>
        </aside>
    }
}

#[component]
fn SidebarLink(href: &'static str, icon: &'static str, label: &'static str) -> impl IntoView {
    let location = use_location();
    let is_active = move || {
        let pathname = location.pathname.get();
        if href == "/" {
            pathname == "/"
        } else {
            pathname.starts_with(href)
        }
    };

    let icon_svg = match icon {
        "dashboard" => view! {
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"></path>
            </svg>
        }.into_any(),
        "chat" => view! {
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
            </svg>
        }.into_any(),
        "documents" => view! {
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
            </svg>
        }.into_any(),
        _ => view! { <span></span> }.into_any(),
    };

    view! {
        <a
            href=href
            class=move || format!(
                "flex items-center gap-3 px-4 py-3 text-sm font-medium rounded-xl transition-colors {}",
                if is_active() {
                    "bg-primary text-primary-foreground"
                } else {
                    "text-muted-foreground hover:text-foreground hover:bg-muted"
                }
            )
        >
            <span class="w-5 h-5 flex items-center justify-center flex-shrink-0">
                {icon_svg}
            </span>
            <span>{label}</span>
        </a>
    }
}

#[component]
pub fn MobileNav() -> impl IntoView {
    let location = use_location();
    let is_active = move |href: &str| {
        let pathname = location.pathname.get();
        if href == "/" {
            pathname == "/"
        } else {
            pathname.starts_with(href)
        }
    };

    view! {
        <div class="md:hidden fixed bottom-0 left-0 right-0 bg-card border-t border-border z-50 safe-area-inset-bottom">
            <nav class="flex justify-evenly py-3">
                <a
                    href="/"
                    class=move || format!(
                        "flex flex-col items-center justify-center gap-1 w-16 py-2 transition-colors {}",
                        if is_active("/") && location.pathname.get() == "/" {
                            "text-primary"
                        } else {
                            "text-muted-foreground hover:text-foreground"
                        }
                    )
                >
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"></path>
                    </svg>
                    <span class="text-xs font-medium">"Home"</span>
                </a>
                <a
                    href="/chat"
                    class=move || format!(
                        "flex flex-col items-center justify-center gap-1 w-16 py-2 transition-colors {}",
                        if is_active("/chat") {
                            "text-primary"
                        } else {
                            "text-muted-foreground hover:text-foreground"
                        }
                    )
                >
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
                    </svg>
                    <span class="text-xs font-medium">"Chat"</span>
                </a>
                <a
                    href="/documents"
                    class=move || format!(
                        "flex flex-col items-center justify-center gap-1 w-16 py-2 transition-colors {}",
                        if is_active("/documents") {
                            "text-primary"
                        } else {
                            "text-muted-foreground hover:text-foreground"
                        }
                    )
                >
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                    </svg>
                    <span class="text-xs font-medium">"Docs"</span>
                </a>
            </nav>
        </div>
    }
}
