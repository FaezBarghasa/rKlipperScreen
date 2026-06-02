use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spool {
    pub id: u32,
    pub name: String,
    pub material: String,
    pub color: String,
    pub remaining_weight: f32,
}

pub struct SpoolmanClient {
    pub enabled: bool,
    pub base_url: String,
}

impl SpoolmanClient {
    pub fn new(enabled: bool, base_url: String) -> Self {
        Self { enabled, base_url }
    }

    pub async fn get_active_spools(&self) -> Result<Vec<Spool>, reqwest::Error> {
        if !self.enabled {
            info!("Spoolman is disabled. Silently bypassing query.");
            return Ok(Vec::new());
        }

        // Mock call / actual REST endpoint if enabled
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/spool", self.base_url);
        let res = client.get(&url).send().await?;
        if res.status().is_success() {
            let spools: Vec<Spool> = res.json().await?;
            Ok(spools)
        } else {
            Ok(Vec::new())
        }
    }
}
