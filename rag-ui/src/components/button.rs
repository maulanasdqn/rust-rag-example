use leptos::prelude::*;

#[component]
pub fn Button(
    #[prop(into)] on_click: Callback<leptos::ev::MouseEvent>,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            on:click=move |ev| {
                if !disabled.get() {
                    on_click.run(ev);
                }
            }
            disabled=disabled
            class="inline-flex items-center justify-center rounded-md text-sm font-medium \
                   ring-offset-background transition-colors \
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
                   disabled:pointer-events-none disabled:opacity-50 \
                   bg-primary text-primary-foreground hover:bg-primary/90 \
                   h-10 px-4 py-2"
        >
            {children()}
        </button>
    }
}
