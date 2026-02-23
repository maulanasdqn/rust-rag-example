mod router;
mod security;
mod use_cases;

use rag_config::Settings;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let settings = Settings::load()?;

    // Log security configuration
    info!(
        "Security: auth_required={}, rate_limit={}/min, expensive_rate_limit={}/min",
        settings.security.require_auth,
        settings.security.rate_limit_rpm,
        settings.security.expensive_rate_limit_rpm
    );

    let state = use_cases::AppState::new(&settings).await?;

    let app = router::create_router(state, settings.security.clone());

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server running on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
