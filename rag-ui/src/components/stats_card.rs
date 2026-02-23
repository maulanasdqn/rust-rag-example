use leptos::prelude::*;

#[component]
pub fn StatsCard<F>(
    title: &'static str,
    value: F,
    #[prop(optional)] description: Option<&'static str>,
    #[prop(optional)] icon: Option<&'static str>,
) -> impl IntoView
where
    F: Fn() -> String + Send + Sync + 'static,
{
    let icon_svg = match icon.unwrap_or("default") {
        "documents" => view! {
            <svg class="w-5 h-5 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
            </svg>
        }.into_any(),
        "chunks" => view! {
            <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2 1 3 3 3h10c2 0 3-1 3-3V7c0-2-1-3-3-3H7C5 4 4 5 4 7zM9 12h6M9 16h6"></path>
            </svg>
        }.into_any(),
        "queries" => view! {
            <svg class="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
        }.into_any(),
        "speed" => view! {
            <svg class="w-5 h-5 text-yellow-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
            </svg>
        }.into_any(),
        _ => view! {
            <svg class="w-5 h-5 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
        }.into_any(),
    };

    view! {
        <div class="bg-card border border-border rounded-xl p-6 hover:shadow-md transition-shadow">
            <div class="flex items-center justify-between">
                <div class="space-y-1">
                    <p class="text-sm font-medium text-muted-foreground">{title}</p>
                    <p class="text-2xl font-bold text-foreground">{move || value()}</p>
                    {description.map(|d| view! {
                        <p class="text-xs text-muted-foreground">{d}</p>
                    })}
                </div>
                <div class="w-12 h-12 rounded-full bg-muted flex items-center justify-center">
                    {icon_svg}
                </div>
            </div>
        </div>
    }
}
