mod router;
mod security;
mod telegram;
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

    // Log Telegram configuration
    if settings.telegram.enabled {
        info!("Telegram bot: enabled");
    } else {
        info!("Telegram bot: disabled");
    }

    let state = use_cases::AppState::new(&settings).await?;

    let app = router::create_router(state.clone(), settings.security.clone());

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("HTTP server running on {}", addr);

    // Run both HTTP server and Telegram bot concurrently
    let telegram_settings = settings.telegram.clone();
    let telegram_state = state.clone();

    tokio::select! {
        result = axum::serve(listener, app) => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
            }
        }
        _ = telegram::run_telegram_bot(telegram_settings, telegram_state) => {
            info!("Telegram bot stopped");
        }
    }

    Ok(())
}
