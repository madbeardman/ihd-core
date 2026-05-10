use std::path::Path;

use chrono::{Local, Timelike};

use crate::agile::{
    build_rolling_window, build_stored_days, fetch_latest_agile_rates, get_agile_window_slots,
    load_stored_day, save_stored_day, stored_day_to_day_slots, RollingSlot, RollingWindow,
};

use crate::home_assistant::{
    extract_live_state, fetch_all_states, is_appliance_running, HaConfig, LiveState,
};
use crate::models::{
    ApplianceRecommendation, ApplianceRecommendations, DashboardState, DeviceCostSummary,
    TopCostDevices, UsageRotationMetrics,
};

type AppError = Box<dyn std::error::Error + Send + Sync>;

pub async fn fetch_and_store_latest_agile(
    agile_dir: &Path,
    ha_config: &HaConfig,
) -> Result<(), AppError> {
    let api = fetch_latest_agile_rates().await?;

    if ha_config.dev_mode {
        println!("Fetched {} raw Agile slots", api.results.len());
    }

    let days = build_stored_days(&api);

    for day in &days {
        let path = save_stored_day(agile_dir, day)?;
        if ha_config.dev_mode {
            println!("Saved {} slots to {}", day.slots.len(), path.display());
        }
    }

    Ok(())
}

pub async fn load_dashboard_state(
    agile_dir: &Path,
    ha_config: &HaConfig,
) -> Result<DashboardState, AppError> {
    let agile = load_rolling_window_from_store(agile_dir)?;

    let live = match fetch_all_states(ha_config).await {
        Ok(states) => extract_live_state(&states),
        Err(err) => {
            eprintln!("Failed to fetch live HA state: {err:?}");
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

    let appliances = build_appliance_recommendations(&live, &agile);
    let usage_metrics = build_usage_rotation_metrics(&live, &agile);

    Ok(DashboardState {
        dev_mode: ha_config.dev_mode,
        live,
        agile,
        appliances,
        usage_metrics,
    })
}

pub fn load_rolling_window_from_store(agile_dir: &Path) -> Result<RollingWindow, AppError> {
    let today = Local::now().date_naive();
    let tomorrow = today.succ_opt().ok_or("Failed to calculate tomorrow")?;

    let today_day = load_stored_day(agile_dir, today)?;
    let tomorrow_day = load_stored_day(agile_dir, tomorrow)?;

    let today_slots = today_day
        .as_ref()
        .map(stored_day_to_day_slots)
        .unwrap_or_default();

    let tomorrow_slots = tomorrow_day
        .as_ref()
        .map(stored_day_to_day_slots)
        .unwrap_or_default();

    let now_local = Local::now();
    let current_slot_index = (now_local.hour() * 2 + now_local.minute() / 30) as u8;

    let rolling_slots = build_rolling_window(
        &today_slots,
        &tomorrow_slots,
        current_slot_index,
        get_agile_window_slots(),
    );

    Ok(RollingWindow {
        current_slot_index,
        slot_count: rolling_slots.len(),
        slots: rolling_slots,
    })
}

pub fn build_usage_rotation_metrics(
    live: &LiveState,
    agile: &RollingWindow,
) -> UsageRotationMetrics {
    let current_power_w = live
        .octopus_current_demand_w
        .or(live.house_power_w)
        .map(|watts| watts.max(0.0));

    let current_price_p_per_kwh = agile.slots.first().map(|slot| slot.value_inc_vat);

    let current_cost_per_hour_gbp = match (current_power_w, current_price_p_per_kwh) {
        (Some(power_w), Some(price_p_per_kwh)) => {
            let pounds_per_kwh = price_p_per_kwh / 100.0;
            Some((power_w / 1000.0) * pounds_per_kwh)
        }
        _ => None,
    };

    UsageRotationMetrics {
        current_power_w,
        current_price_p_per_kwh,
        current_cost_per_hour_gbp,
        cost_today_gbp: live.electricity_cost_today_gbp,
        gas_cost_today_gbp: live.gas_cost_today_gbp,
    }
}

pub fn find_best_start_time(slots: &[RollingSlot], required_slots: usize) -> Option<String> {
    if slots.len() < required_slots || required_slots == 0 {
        return None;
    }

    let mut best_total = f64::MAX;
    let mut best_start = None;

    for window_start in 0..=(slots.len() - required_slots) {
        let window = &slots[window_start..window_start + required_slots];
        let total: f64 = window.iter().map(|slot| slot.value_inc_vat).sum();

        if total < best_total {
            best_total = total;
            best_start = window.first().map(|slot| slot.valid_from);
        }
    }

    best_start.map(|utc_time| utc_time.with_timezone(&Local).format("%H:%M").to_string())
}

pub fn build_appliance_recommendation(
    name: &str,
    power_w: Option<f64>,
    required_slots: usize,
    rolling_slots: &[RollingSlot],
) -> ApplianceRecommendation {
    let running = is_appliance_running(power_w);
    let best_start = if running {
        None
    } else {
        find_best_start_time(rolling_slots, required_slots)
    };

    let display = if running {
        "ON".to_string()
    } else {
        best_start.clone().unwrap_or_else(|| "--".to_string())
    };

    ApplianceRecommendation {
        name: name.to_string(),
        power_w,
        running,
        best_start,
        display,
    }
}

pub fn build_appliance_recommendations(
    live: &LiveState,
    agile: &RollingWindow,
) -> ApplianceRecommendations {
    ApplianceRecommendations {
        dishwasher: build_appliance_recommendation(
            "Dishwasher",
            live.dishwasher_power_w,
            8,
            &agile.slots,
        ),
        washing_machine: build_appliance_recommendation(
            "Washing Machine",
            live.washing_machine_power_w,
            2,
            &agile.slots,
        ),
        tumble_dryer: build_appliance_recommendation(
            "Tumble Dryer",
            live.tumble_dryer_power_w,
            5,
            &agile.slots,
        ),
    }
}

pub fn get_poll_interval_seconds() -> u64 {
    let now = Local::now();
    let hour = now.hour();

    if (7..22).contains(&hour) {
        10
    } else {
        20
    }
}
