use leptos::prelude::*;

#[component]
pub fn Textarea(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_input: Callback<String>,
    #[prop(optional, into, default = String::new())] placeholder: String,
    #[prop(optional, default = 3)] rows: u32,
    #[prop(optional)] on_keydown: Option<Callback<leptos::ev::KeyboardEvent>>,
) -> impl IntoView {
    view! {
        <textarea
            prop:value=value
            on:input=move |ev| {
                let v = event_target_value(&ev);
                on_input.run(v);
            }
            on:keydown=move |ev| {
                if let Some(callback) = on_keydown {
                    callback.run(ev);
                }
            }
            placeholder=placeholder
            rows=rows
            class="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm \
                   ring-offset-background placeholder:text-muted-foreground \
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
                   disabled:cursor-not-allowed disabled:opacity-50 \
                   resize-none"
        />
    }
}
