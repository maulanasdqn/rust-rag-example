use leptos::prelude::*;

#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-card text-card-foreground shadow-sm">
            {children()}
        </div>
    }
}
