use leptos::prelude::*;
use crate::api::{
    list_agents, list_tools, execute_agent, AgentInfo, ToolInfo,
    ReasoningStep, SourceInfo,
};
use crate::components::{Button, Card, Textarea};

#[component]
pub fn AgentsPage() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (agents, set_agents) = signal(Vec::<AgentInfo>::new());
    let (tools, set_tools) = signal(Vec::<ToolInfo>::new());
    let (selected_agent, set_selected_agent) = signal(Option::<String>::None);
    let (result, set_result) = signal(Option::<AgentResult>::None);
    let (error, set_error) = signal(Option::<String>::None);

    // Load agents and tools on mount
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(response) = list_agents().await {
                set_agents.set(response.agents);
            }
            if let Ok(response) = list_tools().await {
                set_tools.set(response.tools);
            }
        });
    });

    let on_submit = move |_| {
        let query_text = query.get();
        if query_text.trim().is_empty() || is_loading.get() {
            return;
        }

        set_is_loading.set(true);
        set_error.set(None);
        set_result.set(None);

        let agent_name = selected_agent.get();

        wasm_bindgen_futures::spawn_local(async move {
            match execute_agent(&query_text, agent_name, None).await {
                Ok(response) => {
                    set_result.set(Some(AgentResult {
                        success: response.success,
                        answer: response.answer,
                        reasoning: response.reasoning,
                        sources: response.sources,
                        error: response.error,
                    }));
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            set_is_loading.set(false);
        });
    };

    view! {
        <div class="flex-1 flex flex-col h-full overflow-hidden">
            <div class="flex-1 overflow-y-auto p-6">
                <div class="max-w-4xl mx-auto space-y-6">
                    // Header
                    <div class="mb-8">
                        <h1 class="text-3xl font-bold text-foreground">"AI Agents"</h1>
                        <p class="text-muted-foreground mt-2">
                            "Execute intelligent agents that can reason, use tools, and provide detailed answers."
                        </p>
                    </div>

                    // Agents and Tools Info Cards
                    <div class="grid md:grid-cols-2 gap-4">
                        // Available Agents
                        <Card>
                            <div class="p-4">
                                <h3 class="font-semibold text-foreground mb-3 flex items-center gap-2">
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                                    </svg>
                                    "Available Agents"
                                </h3>
                                <div class="space-y-2">
                                    {move || {
                                        let agents_list = agents.get();
                                        if agents_list.is_empty() {
                                            view! {
                                                <p class="text-sm text-muted-foreground">"Loading agents..."</p>
                                            }.into_any()
                                        } else {
                                            agents_list.into_iter().map(|agent| {
                                                let agent_name = agent.name.clone();
                                                let is_selected = move || selected_agent.get() == Some(agent_name.clone());
                                                let name_for_click = agent.name.clone();
                                                view! {
                                                    <button
                                                        class=move || format!(
                                                            "w-full text-left p-2 rounded-md transition-colors {}",
                                                            if is_selected() { "bg-primary/20 border border-primary" } else { "hover:bg-muted" }
                                                        )
                                                        on:click=move |_| {
                                                            let current = selected_agent.get();
                                                            if current == Some(name_for_click.clone()) {
                                                                set_selected_agent.set(None);
                                                            } else {
                                                                set_selected_agent.set(Some(name_for_click.clone()));
                                                            }
                                                        }
                                                    >
                                                        <div class="font-medium text-foreground">{agent.name}</div>
                                                        <div class="text-xs text-muted-foreground">{agent.description}</div>
                                                    </button>
                                                }
                                            }).collect_view().into_any()
                                        }
                                    }}
                                </div>
                            </div>
                        </Card>

                        // Available Tools
                        <Card>
                            <div class="p-4">
                                <h3 class="font-semibold text-foreground mb-3 flex items-center gap-2">
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                                    </svg>
                                    "Available Tools"
                                </h3>
                                <div class="space-y-2">
                                    {move || {
                                        let tools_list = tools.get();
                                        if tools_list.is_empty() {
                                            view! {
                                                <p class="text-sm text-muted-foreground">"Loading tools..."</p>
                                            }.into_any()
                                        } else {
                                            tools_list.into_iter().map(|tool| {
                                                view! {
                                                    <div class="p-2 rounded-md bg-muted/50">
                                                        <div class="font-medium text-foreground text-sm">{tool.name}</div>
                                                        <div class="text-xs text-muted-foreground">{tool.description}</div>
                                                    </div>
                                                }
                                            }).collect_view().into_any()
                                        }
                                    }}
                                </div>
                            </div>
                        </Card>
                    </div>

                    // Query Input
                    <Card>
                        <div class="p-4 space-y-4">
                            <h3 class="font-semibold text-foreground">"Ask the Agent"</h3>
                            <Textarea
                                value=query
                                on_input=Callback::new(move |v| set_query.set(v))
                                placeholder="Ask a question... The agent will reason through it step by step."
                            />
                            <div class="flex items-center justify-between">
                                <div class="text-sm text-muted-foreground">
                                    {move || {
                                        match selected_agent.get() {
                                            Some(name) => format!("Using: {}", name),
                                            None => "Auto-select best agent".to_string(),
                                        }
                                    }}
                                </div>
                                <Button
                                    on_click=on_submit
                                    disabled=Signal::derive(move || is_loading.get() || query.get().trim().is_empty())
                                >
                                    {move || if is_loading.get() {
                                        view! {
                                            <span class="flex items-center gap-2">
                                                <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                </svg>
                                                "Thinking..."
                                            </span>
                                        }.into_any()
                                    } else {
                                        view! { <span>"Execute"</span> }.into_any()
                                    }}
                                </Button>
                            </div>
                        </div>
                    </Card>

                    // Error Display
                    {move || error.get().map(|e| view! {
                        <div class="bg-destructive/10 border border-destructive text-destructive rounded-lg p-4">
                            <p class="font-medium">"Error"</p>
                            <p class="text-sm">{e}</p>
                        </div>
                    })}

                    // Result Display
                    {move || result.get().map(|r| view! {
                        <div class="space-y-4">
                            // Reasoning Steps
                            {if !r.reasoning.is_empty() {
                                view! {
                                    <Card>
                                        <div class="p-4">
                                            <h3 class="font-semibold text-foreground mb-3 flex items-center gap-2">
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                                                </svg>
                                                "Reasoning Process"
                                            </h3>
                                            <div class="space-y-2">
                                                {r.reasoning.iter().map(|step| {
                                                    let (icon, bg_class) = match step.step_type.as_str() {
                                                        "thought" => ("💭", "bg-blue-500/10 border-blue-500/30"),
                                                        "action" => ("⚡", "bg-yellow-500/10 border-yellow-500/30"),
                                                        "observation" => ("👁", "bg-green-500/10 border-green-500/30"),
                                                        _ => ("•", "bg-muted"),
                                                    };
                                                    view! {
                                                        <div class=format!("p-3 rounded-md border {}", bg_class)>
                                                            <div class="flex items-start gap-2">
                                                                <span class="text-lg">{icon}</span>
                                                                <div>
                                                                    <div class="text-xs font-medium text-muted-foreground uppercase mb-1">
                                                                        {step.step_type.clone()}
                                                                    </div>
                                                                    <div class="text-sm text-foreground whitespace-pre-wrap">
                                                                        {step.content.clone()}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    </Card>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}

                            // Answer
                            <Card>
                                <div class="p-4">
                                    <h3 class="font-semibold text-foreground mb-3 flex items-center gap-2">
                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                                        </svg>
                                        "Answer"
                                    </h3>
                                    <div class="text-foreground whitespace-pre-wrap">{r.answer.clone()}</div>
                                </div>
                            </Card>

                            // Sources
                            {if !r.sources.is_empty() {
                                view! {
                                    <Card>
                                        <div class="p-4">
                                            <h3 class="font-semibold text-foreground mb-3 flex items-center gap-2">
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                                </svg>
                                                "Sources"
                                            </h3>
                                            <div class="space-y-2">
                                                {r.sources.iter().map(|source| {
                                                    view! {
                                                        <div class="p-3 rounded-md bg-muted/50 border border-border">
                                                            <div class="flex items-center justify-between mb-2">
                                                                <span class="font-medium text-sm text-foreground">{source.source_file.clone()}</span>
                                                                <span class="text-xs text-muted-foreground">
                                                                    {format!("{:.0}% relevant", source.relevance_score * 100.0)}
                                                                </span>
                                                            </div>
                                                            <p class="text-sm text-muted-foreground line-clamp-2">{source.excerpt.clone()}</p>
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    </Card>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}
                        </div>
                    })}
                </div>
            </div>
        </div>
    }
}

#[derive(Clone)]
struct AgentResult {
    success: bool,
    answer: String,
    reasoning: Vec<ReasoningStep>,
    sources: Vec<SourceInfo>,
    error: Option<String>,
}
