use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::settings;
use chrono::{DateTime, Local, NaiveDate, Timelike, Utc};
use serde::{Deserialize, Serialize};

type AppError = Box<dyn std::error::Error + Send + Sync>;

const AGILE_URL: &str = "https://api.octopus.energy/v1/products/AGILE-24-10-01/electricity-tariffs/E-1R-AGILE-24-10-01-M/standard-unit-rates/";

#[derive(Debug, Deserialize)]
pub struct AgileApiResponse {
    pub results: Vec<AgileApiSlot>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgileApiSlot {
    pub value_inc_vat: f64,
    pub valid_from: String,
    pub valid_to: String,
}

#[derive(Debug, Clone)]
pub struct DaySlot {
    pub index: u8,
    pub value_inc_vat: f64,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAgileDay {
    pub date: String,
    pub fetched_at: DateTime<Utc>,
    pub slots: Vec<StoredAgileSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAgileSlot {
    pub index: u8,
    pub value_inc_vat: f64,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PriceBand {
    Negative,
    Cheap,
    Normal,
    Expensive,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceDay {
    Today,
    Tomorrow,
}

impl fmt::Display for SourceDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceDay::Today => write!(f, "today"),
            SourceDay::Tomorrow => write!(f, "tomorrow"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RollingSlot {
    pub offset: u8,
    pub source_day: SourceDay,
    pub source_index: u8,
    pub value_inc_vat: f64,
    pub band: PriceBand,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    pub is_now: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RollingWindow {
    pub current_slot_index: u8,
    pub slot_count: usize,
    pub slots: Vec<RollingSlot>,
}

pub fn classify_price_band(price: f64) -> PriceBand {
    if price < 0.0 {
        PriceBand::Negative
    } else if price < 15.0 {
        PriceBand::Cheap
    } else if price < 25.0 {
        PriceBand::Normal
    } else {
        PriceBand::Expensive
    }
}

pub fn build_rolling_window(
    today_slots: &[DaySlot],
    tomorrow_slots: &[DaySlot],
    current_slot_index: u8,
    max_slots: usize,
) -> Vec<RollingSlot> {
    let mut rolling: Vec<RollingSlot> = Vec::with_capacity(max_slots);

    for slot in today_slots
        .iter()
        .filter(|slot| slot.index >= current_slot_index)
    {
        if rolling.len() >= max_slots {
            break;
        }

        rolling.push(RollingSlot {
            offset: rolling.len() as u8,
            source_day: SourceDay::Today,
            source_index: slot.index,
            value_inc_vat: slot.value_inc_vat,
            band: classify_price_band(slot.value_inc_vat),
            valid_from: slot.valid_from,
            valid_to: slot.valid_to,
            is_now: rolling.is_empty(),
        });
    }

    for slot in tomorrow_slots {
        if rolling.len() >= max_slots {
            break;
        }

        rolling.push(RollingSlot {
            offset: rolling.len() as u8,
            source_day: SourceDay::Tomorrow,
            source_index: slot.index,
            value_inc_vat: slot.value_inc_vat,
            band: classify_price_band(slot.value_inc_vat),
            valid_from: slot.valid_from,
            valid_to: slot.valid_to,
            is_now: rolling.is_empty(),
        });
    }

    rolling
}

pub async fn fetch_latest_agile_rates() -> Result<AgileApiResponse, AppError> {
    let client = reqwest::Client::new();

    let response = client
        .get(AGILE_URL)
        .query(&[("page_size", "150"), ("ordering", "-valid_from")])
        .send()
        .await?
        .error_for_status()?
        .json::<AgileApiResponse>()
        .await?;

    Ok(response)
}

pub fn build_stored_days(api: &AgileApiResponse) -> Vec<StoredAgileDay> {
    let fetched_at = Utc::now();
    let mut grouped: BTreeMap<String, Vec<StoredAgileSlot>> = BTreeMap::new();

    for slot in &api.results {
        let from = DateTime::parse_from_rfc3339(&slot.valid_from)
            .expect("Failed to parse valid_from timestamp")
            .with_timezone(&Utc);

        let to = DateTime::parse_from_rfc3339(&slot.valid_to)
            .expect("Failed to parse valid_to timestamp")
            .with_timezone(&Utc);

        let local_from = from.with_timezone(&Local);
        let local_date = local_from.date_naive();
        let date_key = local_date.format("%Y-%m-%d").to_string();

        let hour = local_from.hour();
        let minute = local_from.minute();
        let index = (hour * 2 + minute / 30) as u8;

        grouped.entry(date_key).or_default().push(StoredAgileSlot {
            index,
            value_inc_vat: slot.value_inc_vat,
            valid_from: from,
            valid_to: to,
        });
    }

    let mut days = Vec::new();

    for (date, mut slots) in grouped {
        slots.sort_by_key(|slot| slot.index);

        days.push(StoredAgileDay {
            date,
            fetched_at,
            slots,
        });
    }

    days
}

pub fn save_stored_day(base_dir: &Path, day: &StoredAgileDay) -> Result<PathBuf, AppError> {
    fs::create_dir_all(base_dir)?;

    let file_path = base_dir.join(format!("{}.json", day.date));
    let json = serde_json::to_string_pretty(day)?;
    fs::write(&file_path, json)?;

    Ok(file_path)
}

pub fn load_stored_day(
    base_dir: &Path,
    date: NaiveDate,
) -> Result<Option<StoredAgileDay>, AppError> {
    let file_path = base_dir.join(format!("{}.json", date.format("%Y-%m-%d")));

    if !file_path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(file_path)?;
    let parsed = serde_json::from_str::<StoredAgileDay>(&data)?;

    Ok(Some(parsed))
}

pub fn stored_day_to_day_slots(day: &StoredAgileDay) -> Vec<DaySlot> {
    let mut slots: Vec<DaySlot> = day
        .slots
        .iter()
        .map(|slot| DaySlot {
            index: slot.index,
            value_inc_vat: slot.value_inc_vat,
            valid_from: slot.valid_from,
            valid_to: slot.valid_to,
        })
        .collect();

    slots.sort_by_key(|slot| slot.index);
    slots
}

pub async fn fetch_agile_rates_for_day(day: NaiveDate) -> Result<AgileApiResponse, AppError> {
    let period_from = day
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid Agile period_from")?;
    let period_to = (day + chrono::Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid Agile period_to")?;

    let client = reqwest::Client::new();

    let response = client
        .get(AGILE_URL)
        .query(&[
            ("page_size", "150"),
            (
                "period_from",
                &period_from.format("%Y-%m-%dT%H:%M:%S").to_string(),
            ),
            (
                "period_to",
                &period_to.format("%Y-%m-%dT%H:%M:%S").to_string(),
            ),
            ("order_by", "period"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<AgileApiResponse>()
        .await?;

    Ok(response)
}

pub async fn fetch_and_store_agile_for_day(
    agile_dir: &Path,
    day: NaiveDate,
) -> Result<(), AppError> {
    let api = fetch_agile_rates_for_day(day).await?;
    let days = build_stored_days(&api);

    for stored_day in &days {
        if stored_day.date == day.format("%Y-%m-%d").to_string() {
            save_stored_day(agile_dir, stored_day)?;
        }
    }

    Ok(())
}

pub fn get_agile_window_slots() -> usize {
    settings::get_agile_window_slots()
}
