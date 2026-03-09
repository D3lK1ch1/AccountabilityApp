use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub app_name: String,
    pub total_seconds: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_tracked_seconds: i64,
    pub most_used_app: Option<String>,
    pub usage_by_app: Vec<UsageData>,
    pub sessions_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerStatus {
    pub is_tracking: bool,
    pub current_app: Option<String>,
    pub current_window_title: Option<String>,
}
