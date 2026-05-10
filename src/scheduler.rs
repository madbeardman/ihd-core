use std::path::PathBuf;
use std::time::Duration;

use chrono::{Datelike, Local, Timelike};

use crate::app_state::AppState;
use crate::dashboard::{
    build_appliance_recommendations, build_usage_rotation_metrics, fetch_and_store_latest_agile,
    get_poll_interval_seconds, load_dashboard_state,
};
use crate::history::{fetch_and_store_yesterday_history, OctopusConfig};
use crate::home_assistant::{extract_live_state, fetch_all_states, log_dev, HaConfig, LiveState};
use crate::models::{DeviceCostSummary, FetchMarker, TopCostDevices};

pub fn start_scheduler(
    state: AppState,
    agile_dir: PathBuf,
    history_dir: PathBuf,
    ha_config: HaConfig,
    octopus_config: Option<OctopusConfig>,
) {
    tokio::spawn(async move {
        let mut last_run: Option<FetchMarker> = None;

        loop {
            let now = Local::now();
            let hour = now.hour();
            let minute = now.minute();

            let should_run_agile = (hour == 4 || hour == 16) && minute == 15;

            if should_run_agile {
                let marker = FetchMarker {
                    year: now.year(),
                    month: now.month(),
                    day: now.day(),
                    hour,
                };

                let already_ran = last_run.as_ref() == Some(&marker);

                if !already_ran {
                    println!(
                        "Scheduled Agile fetch triggered at {:04}-{:02}-{:02} {:02}:{:02}",
                        marker.year, marker.month, marker.day, hour, minute
                    );

                    match fetch_and_store_latest_agile(&agile_dir, &ha_config).await {
                        Ok(()) => {
                            println!("Scheduled Agile fetch/store completed");

                            if let Some(config) = octopus_config.as_ref() {
                                match fetch_and_store_yesterday_history(
                                    &history_dir,
                                    &agile_dir,
                                    config,
                                    ha_config.dev_mode,
                                )
                                .await
                                {
                                    Ok(()) => {
                                        println!("Scheduled history fetch/store completed");
                                    }
                                    Err(err) => {
                                        eprintln!("Scheduled history fetch failed: {err}");
                                    }
                                }
                            } else {
                                println!(
                                    "Octopus config missing — scheduled history fetch skipped"
                                );
                            }

                            last_run = Some(marker);
                        }
                        Err(err) => {
                            eprintln!("Scheduled Agile fetch failed: {err}");
                        }
                    }
                }
            }

            match load_dashboard_state(&agile_dir, &ha_config).await {
                Ok(updated_dashboard) => {
                    let mut guard = state.dashboard.write().await;
                    *guard = updated_dashboard;
                }
                Err(err) => {
                    eprintln!("Failed to refresh dashboard state: {err}");
                }
            }

            let poll_interval = get_poll_interval_seconds();
            tokio::time::sleep(Duration::from_secs(poll_interval)).await;
        }
    });
}

pub fn start_home_assistant_polling(state: AppState, ha_config: HaConfig) {
    tokio::spawn(async move {
        loop {
            let now = chrono::Local::now().format("%H:%M:%S");

            let live = match fetch_all_states(&ha_config).await {
                Ok(states) => extract_live_state(&states),
                Err(err) => {
                    eprintln!("[{now}] HA fetch failed: {err:?}");
                    LiveState {
                        house_power_w: None,
                        solar_generation_w: None,
                        dishwasher_power_w: None,
                        washing_machine_power_w: None,
                        tumble_dryer_power_w: None,
                        electricity_cost_today_gbp: None,
                        octopus_current_demand_w: None,
                        gas_cost_today_gbp: None,
                        device_costs: DeviceCostSummary {
                            current: TopCostDevices { items: vec![] },
                            today: TopCostDevices { items: vec![] },
                            yesterday: TopCostDevices { items: vec![] },
                            month: TopCostDevices { items: vec![] },
                        },
                    }
                }
            };

            {
                let mut dashboard = state.dashboard.write().await;
                dashboard.live = live.clone();
                dashboard.appliances =
                    build_appliance_recommendations(&dashboard.live, &dashboard.agile);
                dashboard.usage_metrics =
                    build_usage_rotation_metrics(&dashboard.live, &dashboard.agile);
            }

            let house_text = live
                .house_power_w
                .map(|v| format!("{v:.2}W"))
                .unwrap_or_else(|| "unavailable".to_string());

            let solar_text = live
                .solar_generation_w
                .map(|v| format!("{v:.2}W"))
                .unwrap_or_else(|| "unavailable".to_string());

            let dishwasher_text = live
                .dishwasher_power_w
                .map(|v| format!("{v:.2}W"))
                .unwrap_or_else(|| "unavailable".to_string());

            let washer_text = live
                .washing_machine_power_w
                .map(|v| format!("{v:.2}W"))
                .unwrap_or_else(|| "unavailable".to_string());

            let dryer_text = live
                .tumble_dryer_power_w
                .map(|v| format!("{v:.2}W"))
                .unwrap_or_else(|| "unavailable".to_string());

            log_dev(
                &ha_config,
                format!(
                    "[{now}] HA poll | house: {house_text} | solar: {solar_text} | dishwasher: {dishwasher_text} | washer: {washer_text} | dryer: {dryer_text}"
                ),
            );

            let poll_interval = get_poll_interval_seconds();
            tokio::time::sleep(Duration::from_secs(poll_interval)).await;
        }
    });
}
