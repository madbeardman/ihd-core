use std::path::PathBuf;
use std::sync::Arc;

use axum::http::Method;
use axum::{routing::get, Router};
use dotenvy::dotenv;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use agile_fetcher::app_state::AppState;
use agile_fetcher::app_state::RefreshState;
use agile_fetcher::dashboard::{fetch_and_store_latest_agile, load_dashboard_state};
use agile_fetcher::handlers::{
    get_agile, get_dashboard, get_history_day, get_history_month, get_history_week,
    get_history_yesterday, get_settings, index, update_settings,
};
use agile_fetcher::history;
use agile_fetcher::home_assistant::load_ha_config;
use agile_fetcher::scheduler::{start_home_assistant_polling, start_scheduler};
use agile_fetcher::settings;

#[tokio::main]
async fn main() {
    dotenv().ok();

    settings::ensure_settings_file().expect("Failed to initialise settings file");

    let agile_dir = PathBuf::from("data/agile");
    let history_dir = PathBuf::from("data/history");

    let ha_config = load_ha_config().expect("Failed to load Home Assistant config");
    let octopus_config = history::load_octopus_config().ok();

    fetch_and_store_latest_agile(&agile_dir, &ha_config)
        .await
        .expect("Failed to fetch/store Agile data at startup");

    if let Some(config) = octopus_config.as_ref() {
        history::fetch_and_store_yesterday_history(
            &history_dir,
            &agile_dir,
            config,
            ha_config.dev_mode,
        )
        .await
        .expect("Failed to fetch/store yesterday history");
    } else {
        println!("Octopus config missing — history disabled");
    }

    let dashboard = load_dashboard_state(&agile_dir, &ha_config)
        .await
        .expect("Failed to load dashboard state");

    println!(
        "Loaded dashboard with {} Agile slots",
        dashboard.agile.slot_count
    );

    let state = AppState {
        dashboard: Arc::new(RwLock::new(dashboard)),
        history_dir: history_dir.clone(),
        agile_dir: agile_dir.clone(),
        ha_config: ha_config.clone(),
        refresh: Arc::new(RwLock::new(RefreshState {
            agile_refresh_in_progress: false,
            last_agile_refresh_attempt_at: None,
        })),
    };

    start_home_assistant_polling(state.clone(), ha_config.clone());
    start_scheduler(
        state.clone(),
        agile_dir.clone(),
        history_dir.clone(),
        ha_config.clone(),
        octopus_config.clone(),
    );

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/dashboard", get(get_dashboard))
        .route("/api/agile", get(get_agile))
        .route("/api/settings", get(get_settings).post(update_settings))
        .route("/api/history/yesterday", get(get_history_yesterday))
        .route("/api/history/day", get(get_history_day))
        .route("/api/history/week", get(get_history_week))
        .route("/api/history/month", get(get_history_month))
        .route("/", get(index))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state.clone())
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind server");

    println!("----------------------------------------");
    println!("Frontend:      http://0.0.0.0:3000");
    println!("Dashboard API: http://0.0.0.0:3000/api/dashboard");
    println!("Agile API:     http://0.0.0.0:3000/api/agile");
    println!("Settings API:  http://0.0.0.0:3000/api/settings");
    if octopus_config.is_some() {
        println!("History API:   http://0.0.0.0:3000/api/history/yesterday");
    } else {
        println!("History API:   disabled (missing Octopus config)");
    }
    println!("History Day:   http://0.0.0.0:3000/api/history/day?date=2026-04-01");
    println!("History Week:  http://0.0.0.0:3000/api/history/week?date=2026-04-01");
    println!("History Month: http://0.0.0.0:3000/api/history/month?date=2026-04-01");
    println!(
        "Logging:       {}",
        if ha_config.dev_mode {
            "ENABLED (DEV_MODE=true)"
        } else {
            "disabled (DEV_MODE=false)"
        }
    );
    println!("----------------------------------------");

    axum::serve(listener, app).await.expect("Server failed");
}
