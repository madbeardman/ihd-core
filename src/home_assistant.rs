use std::cmp::Ordering;

use serde::Deserialize;
use serde_json::Value;

use crate::models::{CostDeviceItem, DeviceCostSummary, TopCostDevices};

type AppError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct HaConfig {
    pub base_url: String,
    pub token: String,
    pub dev_mode: bool,
}

#[derive(Debug, Deserialize)]
pub struct HaState {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveState {
    pub house_power_w: Option<f64>,
    pub solar_generation_w: Option<f64>,
    pub dishwasher_power_w: Option<f64>,
    pub washing_machine_power_w: Option<f64>,
    pub tumble_dryer_power_w: Option<f64>,
    pub device_costs: DeviceCostSummary,
    pub electricity_cost_today_gbp: Option<f64>,
    pub octopus_current_demand_w: Option<f64>,
    pub gas_cost_today_gbp: Option<f64>,
}

fn parse_top_cost_devices(attributes: &serde_json::Map<String, Value>) -> TopCostDevices {
    let mut items = Vec::new();

    for i in 1..=5 {
        let name_key = format!("top_{}_name", i);
        let cost_key = format!("top_{}_cost", i);

        let name = attributes
            .get(&name_key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        let cost = attributes
            .get(&cost_key)
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
            })
            .unwrap_or(0.0);

        if !name.is_empty() {
            items.push(CostDeviceItem {
                name,
                cost_gbp: cost,
            });
        }
    }

    items.sort_by(|a, b| {
        b.cost_gbp
            .partial_cmp(&a.cost_gbp)
            .unwrap_or(Ordering::Equal)
    });

    TopCostDevices { items }
}

fn empty_top_cost_devices() -> TopCostDevices {
    TopCostDevices { items: Vec::new() }
}

pub fn load_ha_config() -> Result<HaConfig, Box<dyn std::error::Error>> {
    let base_url = std::env::var("HA_BASE_URL")?;
    let token = std::env::var("HA_TOKEN")?;

    let dev_mode = std::env::var("DEV_MODE")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        == "true";

    Ok(HaConfig {
        base_url,
        token,
        dev_mode,
    })
}

pub async fn fetch_all_states(config: &HaConfig) -> Result<Vec<HaState>, AppError> {
    let url = format!("{}/api/states", config.base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .bearer_auth(&config.token)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<HaState>>()
        .await?;

    Ok(response)
}

pub fn get_numeric_state(states: &[HaState], entity_id: &str) -> Option<f64> {
    states
        .iter()
        .find(|state| state.entity_id == entity_id)
        .and_then(|state| state.state.parse::<f64>().ok())
}

pub fn extract_live_state(states: &[HaState]) -> LiveState {
    let current_costs = states
        .iter()
        .find(|state| state.entity_id == "sensor.top_cost_devices_current")
        .map(|state| parse_top_cost_devices(&state.attributes))
        .unwrap_or_else(empty_top_cost_devices);

    let today_costs = states
        .iter()
        .find(|state| state.entity_id == "sensor.top_cost_devices_today")
        .map(|state| parse_top_cost_devices(&state.attributes))
        .unwrap_or_else(empty_top_cost_devices);

    let yesterday_costs = states
        .iter()
        .find(|state| state.entity_id == "sensor.top_cost_devices_yesterday")
        .map(|state| parse_top_cost_devices(&state.attributes))
        .unwrap_or_else(empty_top_cost_devices);

    let month_costs = states
        .iter()
        .find(|state| state.entity_id == "sensor.top_cost_devices_month")
        .map(|state| parse_top_cost_devices(&state.attributes))
        .unwrap_or_else(empty_top_cost_devices);

    LiveState {
        house_power_w: get_numeric_state(states, "sensor.total_power_being_used"),
        solar_generation_w: get_numeric_state(states, "sensor.solar_panel_led_sensor_power"),
        dishwasher_power_w: get_numeric_state(states, "sensor.dishwasher_power"),
        washing_machine_power_w: get_numeric_state(states, "sensor.washing_machine_power"),
        tumble_dryer_power_w: get_numeric_state(states, "sensor.tumble_dryer_power"),
        electricity_cost_today_gbp: get_numeric_state(
            states,
            "sensor.octopus_energy_electricity_21e5386139_2334051220712_current_accumulative_cost",
        ),
        gas_cost_today_gbp: get_numeric_state(
            states,
            "sensor.octopus_energy_gas_e6e02565312021_2158072603_current_accumulative_cost",
        ),
        octopus_current_demand_w: get_numeric_state(
            states,
            "sensor.octopus_energy_electricity_21e5386139_2334051220712_current_demand",
        ),
        device_costs: DeviceCostSummary {
            current: current_costs,
            today: today_costs,
            yesterday: yesterday_costs,
            month: month_costs,
        },
    }
}

pub fn is_appliance_running(power_w: Option<f64>) -> bool {
    power_w.unwrap_or(0.0) > 2.0
}

pub fn log_dev(config: &HaConfig, message: impl AsRef<str>) {
    if config.dev_mode {
        println!("{}", message.as_ref());
    }
}
