use axum::{
    extract::{Query, State},
    response::Html,
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::agile::RollingWindow;
use crate::app_state::AppState;
use crate::dashboard::load_dashboard_state;
use crate::history::{
    load_history_for_day, load_history_for_month, load_history_for_week, load_yesterday_history,
    MonthHistoryResponse, WeekHistoryResponse, YesterdayHistoryResponse,
};
use crate::models::DashboardState;
use crate::refresh::trigger_agile_refresh_if_needed;
use crate::settings::{load_settings, save_settings, AppSettings};

#[derive(Debug, Deserialize)]
pub struct HistoryDayQuery {
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub agile_window_slots: usize,
}

pub async fn get_dashboard(State(state): State<AppState>) -> Json<DashboardState> {
    let dashboard = {
        let guard = state.dashboard.read().await;
        guard.clone()
    };

    trigger_agile_refresh_if_needed(state.clone(), dashboard.agile.slots.len()).await;

    Json(dashboard)
}

pub async fn get_agile(State(state): State<AppState>) -> Json<RollingWindow> {
    let agile = {
        let dashboard = state.dashboard.read().await;
        dashboard.agile.clone()
    };

    trigger_agile_refresh_if_needed(state.clone(), agile.slots.len()).await;

    Json(agile)
}

pub async fn get_settings() -> Result<Json<AppSettings>, (axum::http::StatusCode, String)> {
    let settings = load_settings().map_err(|err| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load settings: {err}"),
        )
    })?;

    Ok(Json(settings))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<AppSettings>, (axum::http::StatusCode, String)> {
    let settings = AppSettings {
        agile_window_slots: payload.agile_window_slots,
    };

    let saved = save_settings(&settings).map_err(|err| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save settings: {err}"),
        )
    })?;

    let refreshed_dashboard = load_dashboard_state(&state.agile_dir, &state.ha_config)
        .await
        .map_err(|err| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to rebuild dashboard after settings save: {err}"),
            )
        })?;

    {
        let mut dashboard = state.dashboard.write().await;
        *dashboard = refreshed_dashboard;
    }

    Ok(Json(saved))
}

pub async fn get_history_yesterday(
    State(state): State<AppState>,
) -> Json<YesterdayHistoryResponse> {
    let history = load_yesterday_history(&state.history_dir).unwrap_or(YesterdayHistoryResponse {
        electricity: None,
        gas: None,
    });

    Json(history)
}

pub async fn get_history_day(
    State(state): State<AppState>,
    Query(query): Query<HistoryDayQuery>,
) -> Result<Json<YesterdayHistoryResponse>, (axum::http::StatusCode, String)> {
    let day = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d").map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid date format: {}. Expected YYYY-MM-DD", query.date),
        )
    })?;

    let history = load_history_for_day(&state.history_dir, day).map_err(|err| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load history for day: {err}"),
        )
    })?;

    Ok(Json(history))
}

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let html = include_str!("../static/index.html");

    let dev_mode = {
        let dashboard = state.dashboard.read().await;
        if dashboard.dev_mode {
            "true"
        } else {
            "false"
        }
    };

    Html(html.replace(
        r#"data-dev-mode="false""#,
        &format!(r#"data-dev-mode="{}""#, dev_mode),
    ))
}

pub async fn get_history_week(
    State(state): State<AppState>,
    Query(query): Query<HistoryDayQuery>,
) -> Result<Json<WeekHistoryResponse>, (axum::http::StatusCode, String)> {
    let day = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d").map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid date format: {}. Expected YYYY-MM-DD", query.date),
        )
    })?;

    let history = load_history_for_week(&state.history_dir, day).map_err(|err| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load history for week: {err}"),
        )
    })?;

    Ok(Json(history))
}

pub async fn get_history_month(
    State(state): State<AppState>,
    Query(query): Query<HistoryDayQuery>,
) -> Result<Json<MonthHistoryResponse>, (axum::http::StatusCode, String)> {
    let day = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d").map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid date format: {}. Expected YYYY-MM-DD", query.date),
        )
    })?;

    let history = load_history_for_month(&state.history_dir, day).map_err(|err| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load history for month: {err}"),
        )
    })?;

    Ok(Json(history))
}
