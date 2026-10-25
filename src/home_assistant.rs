use std::cmp::Ordering;

use serde::Deserialize;
use serde_json::Value;

use crate::models::{
    CostDeviceItem, DeviceCostSummary, DevicePowerSummary, PowerDeviceItem, TopCostDevices,
    TopPowerDevices,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Idle,
    Unavailable,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveState {
    pub house_power_w: Option<f64>,
    pub solar_generation_w: Option<f64>, // Solis S6-GR1P - Main solar generation
    pub solar_hybrid_w: Option<f64>,     // Solis S6-EH1P - Additional solar generation
    pub total_solar_generation_w: Option<f64>, // Combined solar generation (main + additional)

    // Solis S6-EH1P - Dyness battery
    pub battery_soc_percent: Option<f64>,
    pub battery_charge_w: Option<f64>,
    pub battery_discharge_w: Option<f64>,
    pub battery_status: BatteryStatus,

    pub dishwasher_power_w: Option<f64>,
    pub washing_machine_power_w: Option<f64>,
    pub tumble_dryer_power_w: Option<f64>,
    pub consoles_power_w: Option<f64>,
    pub device_costs: DeviceCostSummary,
    pub device_power: DevicePowerSummary,
    pub electricity_cost_today_gbp: Option<f64>,
    pub octopus_current_demand_w: Option<f64>,
    pub grid_power_net_w: Option<f64>,
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

    let current_power = build_current_power_devices(states);

    let solar_generation_w = get_numeric_state(states, "sensor.solis_s6_gr1p_active_power");

    let solar_hybrid_w = get_numeric_state(states, "sensor.solis_s6_eh1p_total_pv_power");

    let total_solar_generation_w = match (solar_generation_w, solar_hybrid_w) {
        (Some(original), Some(hybrid)) => Some(original + hybrid),
        _ => None,
    };

    let battery_charge_w = get_numeric_state(states, "sensor.solis_s6_eh1p_battery_charge_power");
    let battery_discharge_w =
        get_numeric_state(states, "sensor.solis_s6_eh1p_battery_discharge_power");
    let battery_status = calculate_battery_status(battery_charge_w, battery_discharge_w);

    LiveState {
        house_power_w: get_numeric_state(states, "sensor.total_power_being_used"),

        // Solar generation - Original Solis inverter
        solar_generation_w: get_numeric_state(states, "sensor.solis_s6_gr1p_active_power"),

        // Solar generation - New Solis S6-EH1P inverter (PV2)
        solar_hybrid_w: get_numeric_state(states, "sensor.solis_s6_eh1p_total_pv_power"),

        total_solar_generation_w: total_solar_generation_w,

        // Dyness battery - State of charge (%)
        battery_soc_percent: get_numeric_state(states, "sensor.solis_s6_eh1p_battery_soc"),

        // Dyness battery - Charging power (W)
        battery_charge_w: battery_charge_w,

        // Dyness battery - Discharging power (W)
        battery_discharge_w: battery_discharge_w,

        // Dyness battery - Status (Charging, Discharging, Idle, Unavailable)
        battery_status: battery_status,

        // Appliances
        dishwasher_power_w: get_numeric_state(states, "sensor.dishwasher_power"),
        washing_machine_power_w: get_numeric_state(states, "sensor.washing_machine_power"),
        tumble_dryer_power_w: get_numeric_state(states, "sensor.tumble_dryer_power"),
        consoles_power_w: get_numeric_state(states, "sensor.generic_plug_power"),

        // Octopus Energy
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

        grid_power_net_w: get_numeric_state(states, "sensor.solis_s6_eh1p_grid_power_net"),

        // Device summaries
        device_costs: DeviceCostSummary {
            current: current_costs,
            today: today_costs,
            yesterday: yesterday_costs,
            month: month_costs,
        },

        device_power: DevicePowerSummary {
            current: current_power,
            today: empty_top_power_devices(),
            yesterday: empty_top_power_devices(),
            month: empty_top_power_devices(),
        },
    }
}

pub fn calculate_battery_status(charge_w: Option<f64>, discharge_w: Option<f64>) -> BatteryStatus {
    const THRESHOLD_W: f64 = 20.0;

    let (Some(charge), Some(discharge)) = (charge_w, discharge_w) else {
        return BatteryStatus::Unavailable;
    };

    if !charge.is_finite() || !discharge.is_finite() {
        return BatteryStatus::Unavailable;
    }

    if charge < 0.0 || discharge < 0.0 {
        return BatteryStatus::Unavailable;
    }

    if charge > THRESHOLD_W && discharge > THRESHOLD_W {
        return BatteryStatus::Unavailable;
    }

    if charge > THRESHOLD_W {
        BatteryStatus::Charging
    } else if discharge > THRESHOLD_W {
        BatteryStatus::Discharging
    } else {
        BatteryStatus::Idle
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

fn empty_top_power_devices() -> TopPowerDevices {
    TopPowerDevices { items: Vec::new() }
}

fn power_item(name: &str, power_w: Option<f64>) -> Option<PowerDeviceItem> {
    let power_w = power_w.unwrap_or(0.0);

    if power_w <= 0.0 {
        return None;
    }

    Some(PowerDeviceItem {
        name: name.to_string(),
        power_w,
    })
}

fn build_current_power_devices(states: &[HaState]) -> TopPowerDevices {
    let mut items = Vec::new();

    let devices = [
        ("Network Hub", "sensor.network_hub_power"),
        ("Office Hub", "sensor.office_hub_power"),
        ("TV Hub", "sensor.tv_hub_power"),
        ("Music Centre", "sensor.tv_accessories_hub_power"),
        ("Dishwasher", "sensor.dishwasher_power"),
        ("Washing Machine", "sensor.washing_machine_power"),
        ("Tumble Dryer", "sensor.tumble_dryer_power"),
        ("Garage Fridge", "sensor.garage_fridge_power"),
        ("Consoles", "sensor.generic_plug_power"),
        ("Kitchen Fridge", "sensor.kitchen_fridge_power"),
        (
            "Kitchen Small Appliances",
            "sensor.kitchen_small_appliances_power",
        ),
    ];

    for (name, entity_id) in devices {
        if let Some(item) = power_item(name, get_numeric_state(states, entity_id)) {
            items.push(item);
        }
    }

    items.sort_by(|a, b| b.power_w.partial_cmp(&a.power_w).unwrap_or(Ordering::Equal));

    TopPowerDevices { items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_charging() {
        assert_eq!(
            calculate_battery_status(Some(234.0), Some(0.0)),
            BatteryStatus::Charging
        );
    }

    #[test]
    fn battery_discharging() {
        assert_eq!(
            calculate_battery_status(Some(0.0), Some(450.0)),
            BatteryStatus::Discharging
        );
    }

    #[test]
    fn battery_idle() {
        assert_eq!(
            calculate_battery_status(Some(0.0), Some(0.0)),
            BatteryStatus::Idle
        );
    }

    #[test]
    fn battery_below_threshold() {
        assert_eq!(
            calculate_battery_status(Some(15.0), Some(0.0)),
            BatteryStatus::Idle
        );
    }

    #[test]
    fn battery_at_threshold() {
        assert_eq!(
            calculate_battery_status(Some(20.0), Some(0.0)),
            BatteryStatus::Idle
        );
    }

    #[test]
    fn battery_unavailable() {
        assert_eq!(
            calculate_battery_status(None, Some(0.0)),
            BatteryStatus::Unavailable
        );
    }

    #[test]
    fn battery_invalid_reading() {
        assert_eq!(
            calculate_battery_status(Some(f64::NAN), Some(0.0)),
            BatteryStatus::Unavailable
        );
    }

    #[test]
    fn battery_conflicting_readings() {
        assert_eq!(
            calculate_battery_status(Some(200.0), Some(150.0)),
            BatteryStatus::Unavailable
        );
    }
}
