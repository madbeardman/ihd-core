use chrono::{Duration, Local, Timelike, Utc};
use tokio::task;

use crate::agile::fetch_and_store_agile_for_day;
use crate::app_state::AppState;
use crate::dashboard::load_dashboard_state;

const MIN_ACCEPTABLE_AGILE_SLOTS: usize = 36; // 18 hours
const AGILE_REFRESH_COOLDOWN_SECONDS: i64 = 15 * 60;

pub async fn trigger_agile_refresh_if_needed(state: AppState, current_slot_count: usize) {
    // Before the afternoon Agile publication window, it is normal to have fewer future slots.
    let hour = Local::now().hour();
    if hour < 16 {
        return;
    }

    if current_slot_count >= MIN_ACCEPTABLE_AGILE_SLOTS {
        return;
    }

    {
        let mut refresh = state.refresh.write().await;

        if refresh.agile_refresh_in_progress {
            return;
        }

        if let Some(last_attempt) = refresh.last_agile_refresh_attempt_at {
            let elapsed = Utc::now() - last_attempt;

            if elapsed < Duration::seconds(AGILE_REFRESH_COOLDOWN_SECONDS) {
                return;
            }
        }

        refresh.agile_refresh_in_progress = true;
        refresh.last_agile_refresh_attempt_at = Some(Utc::now());
    }

    task::spawn(async move {
        let today = Local::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);

        println!(
            "Lazy Agile refresh triggered: rolling window has fewer than {} slots",
            MIN_ACCEPTABLE_AGILE_SLOTS
        );

        let today_result = fetch_and_store_agile_for_day(&state.agile_dir, today).await;
        let tomorrow_result = fetch_and_store_agile_for_day(&state.agile_dir, tomorrow).await;

        if let Err(err) = today_result {
            eprintln!("Lazy Agile refresh failed for today: {err}");
        }

        if let Err(err) = tomorrow_result {
            eprintln!("Lazy Agile refresh failed for tomorrow: {err}");
        }

        match load_dashboard_state(&state.agile_dir, &state.ha_config).await {
            Ok(updated_dashboard) => {
                let mut dashboard = state.dashboard.write().await;
                *dashboard = updated_dashboard;
                println!("Dashboard state reloaded after lazy Agile refresh");
            }
            Err(err) => {
                eprintln!("Failed to reload dashboard after lazy Agile refresh: {err}");
            }
        }

        {
            let mut refresh = state.refresh.write().await;
            refresh.agile_refresh_in_progress = false;
        }

        println!("Lazy Agile refresh completed");
    });
}
