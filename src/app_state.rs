use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::home_assistant::HaConfig;
use crate::models::DashboardState;

#[derive(Debug, Clone)]
pub struct RefreshState {
    pub agile_refresh_in_progress: bool,
    pub last_agile_refresh_attempt_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct AppState {
    pub dashboard: Arc<RwLock<DashboardState>>,
    pub refresh: Arc<RwLock<RefreshState>>,
    pub history_dir: PathBuf,
    pub agile_dir: PathBuf,
    pub ha_config: HaConfig,
}
